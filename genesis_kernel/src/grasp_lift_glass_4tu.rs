#![no_implicit_prelude]
//! A glass-faced object lifted from a table between two finger pads: the
//! company's own specimen object, 0.2 kg with two flat vertical glass faces
//! 60 mm tall, held at the middle of its faces in a two-finger grip whose
//! pads are those of the 4TU set, human index finger pads on glass.
//!
//! The pads and their data. Willemet, Huloux and Wiertlewski, "Efficient
//! tactile encoding of object slippage", Sci. Rep. 12 (2022), doi
//! 10.1038/s41598-022-16938-1, data on 4TU.ResearchData, doi
//! 10.4121/19329506.v1, both CC BY 4.0. Twelve people's index finger pads,
//! each held at about 20 degrees to a glass plate pressed on it by a lever
//! and weights at about 1.2 N, the plate slid at 10 mm/s until the whole
//! contact slides, seven levels of ultrasonic lubrication, four repeats; force
//! at 300 samples a second. The record in the research folder is
//! `evidence/research/grasp_slip_4tu/`.
//!
//! The two fitted constants. [`FRICTION_COEFFICIENT`] is fitted to the
//! development trials on natural glass, friction level 1: the geometric mean
//! over the 36 trials of the mean lateral force over the mean normal force of
//! each trial's full slip. [`PAD_TANGENTIAL_STIFFNESS_N_M`] is the geometric
//! mean over the same trials of the stiffness at which this card's contact,
//! replayed on each trial's own grip and friction, reaches full slip at the
//! trial's own plate travel. The development trials are subject S1 whole and
//! S4 to S11 at levels 1, 2, 3, 5, 6 and 7; held back from every fit are S2,
//! S3 and S12 whole and level 4 of S4 to S11, chosen by a seed before their
//! files were fetched. The object's mass and faces are the company's own.
//!
//! What the development trials showed. The friction ratio at the onset of
//! full slip is the ratio while sliding, or a few percent under it: one
//! coefficient serves for static and kinetic friction, and no rate law is
//! carried. Within one finger the friction force grows as the grip to the
//! power 0.61, the ratio falling as the grip rises; across fingers the
//! fingers' own levels differ by more, and the friction force of a finger
//! the fit never saw is predicted best by one coefficient, Coulomb's law
//! (14.5 percent median error leaving each finger out, against 19.7 for the
//! power within a finger). The card carries Coulomb's law. The stiffness at
//! which the replay reaches full slip falls with the ultrasonic lubrication,
//! from 340 N/m on natural glass to 163 N/m at level 7: the travel to full
//! slip does not shrink in proportion to the friction force, as
//! Cattaneo-Mindlin's would. The card's stiffness is natural glass's.
//!
//! The comparison with the held-back trials, made once on 2026-10-15 through
//! the engine, by lines fixed before their files were fetched (pre-
//! registration SHA-256
//! 51df1473e92b2182863ac8a30f9520368bce9ae3fd7e30b3dffe4ae3a45410b6). One of
//! four lines met, as forecast. Met: within a trial the lateral force at the
//! onset of full slip is the trial's sliding ratio times its normal force,
//! median error 6.7 percent over 116 trials (line 10 percent). Missed: the
//! stuck-fraction law against the measured stick ratio at 1 mm, root mean
//! square 0.389 against 0.10, the stick ratio equal to the safety margin
//! doing better at 0.334; the card's friction on the 12 trials of three new
//! fingers on natural glass, median error 22.2 percent against 10, a
//! constant force 31.5 and each finger's own coefficient 8.8; and the
//! plate travel to full slip on them, 38.1 percent against 18, a constant
//! travel 33.8. The record is `evidence/research/grasp_slip_4tu/RESULT.md`.
//!
//! The contact. Each pad is a Cattaneo-Mindlin contact, an elastic round
//! contact under a sideways load below its friction force: the stuck centre
//! shrinks from the edge as the load rises, the ring outside it slides, and
//! the whole contact slides at the friction force. It is carried as
//! [`ELEMENTS`] springs with sliders, one for each equal-area slice of the
//! stuck centre, the outermost first. With `u_j = sqrt(1 − j/n)`, element `j`
//! has the stiffness share `u_{j−1} − u_j` of the pad's stiffness `K` and the
//! strength share `1.5 [g(u_{j−1}) − g(u_j)]`, `g(u) = u − u³/3`, of the
//! pad's friction force `F_s`. On a first loading the sum of the elements is
//! Cattaneo-Mindlin's
//!
//! ```text
//! Q = F_s [1 − (1 − d/d*)^(3/2)],    d* = 1.5 F_s / K
//! stuck fraction = (1 − Q/F_s)^(2/3)
//! ```
//!
//! at the ends of the slices, and a load turned back follows Mindlin and
//! Deresiewicz's law of reversed slip from the edge, with no rule added. The
//! stuck fraction is the share of elements that did not slide in the step.
//! When the grip falls, an element above its new strength slides back to it
//! at once, and the stored energy it gives up is heat.
//!
//! The friction force and the stiffness, per pad, with `N` the grip force:
//!
//! ```text
//! F_s = FRICTION_COEFFICIENT · N
//! K   = PAD_TANGENTIAL_STIFFNESS
//! ```
//!
//! The object. Its height `z` above the table and its velocity `v`; the
//! hand's height `z_h` and velocity `v_h`, driven by the caller's
//! acceleration `a_h`, held across the step. The pads move with the hand; the
//! travel of a pad along the faces is `r = z_h − z`, zero at the grip. With
//! both pads carrying `2Q`:
//!
//! ```text
//! m dv/dt = 2Q − m g + R          R ≥ 0, the table's push, only while the object rests on it
//! ```
//!
//! The step. The hand moves exactly, `z_h += v_h h + a_h h²/2`. The object
//! moves by the trapezoid rule, `z += h (v + v')/2`, with
//! `m (v' − v) = h (2Q̄ − m g)`, where `Q̄` is the mean force of a pad over the
//! travel `Δ` of the step, each element followed exactly along it. The
//! equation is solved exactly: each element's force is linear in `Δ` until
//! it reaches its strength and constant after, so between the points where
//! elements yield the equation is a quadratic, solved in closed form. While
//! the object rests on the table it stays there unless the pads' mean force
//! over the step lifts it.
//!
//! The ledgers, held at every step:
//!
//! ```text
//! hand work     W = W_in − W_out,  W_in += 2 Q̄ Δz_h where it is positive, W_out += −2 Q̄ Δz_h where it is negative
//! friction heat H += 2 Σ f_j |slide_j|       never falls
//! energy defect   = (½ m v² + m g z + E_springs) − E_anchor + H − W
//! momentum defect = m v − p_anchor − Σ h (2 Q̄ − m g + R̄)
//! ```
//!
//! The work of the hand is the kinetic and potential energy of the object,
//! the energy in the pads' springs and the heat of the sliding. Each defect
//! is held to [`RESIDUAL_CONSERVATION_LIMIT`] of what is in play in its
//! ledger, and never of less than a floor:
//!
//! ```text
//! energy scale   = max(2 m g H,        |E_anchor| + ½ m v² + |m g z| + E_springs + H + W_in + W_out)
//! momentum scale = max(m sqrt(4 g H),  |m v| + |p_anchor| + |contact impulse| + |gravity impulse| + |table impulse|)
//! ```
//!
//! The floors are the potential energy of the object over the height of its
//! faces, `2 m g H`, and the momentum of a fall from that height. The hand's
//! work is booked in two sums, what it gives and what it takes back, so that
//! a lift and a lowering of the same height both stay in play: the rounding
//! of a run grows with what its sums have carried, and a scale that fell
//! with the net work would refuse an honest run. A positive energy defect is
//! energy from nowhere and is [`StepReject::SpontaneousGeneration`]; a
//! negative one is [`StepReject::FirstLawBreach`]; a momentum defect past
//! its limit is [`StepReject::MomentumLedgerBroken`]. A step whose heat falls
//! is [`StepReject::FrictionHeatNegative`].
//!
//! The limits are physical. No grip force is no contact
//! ([`StepReject::NoContact`]). An object below the table's surface is no
//! state ([`StepReject::ObjectBelowTable`]). The pads slid down to the lower
//! end of the faces of an object resting on the table are at the table's
//! surface: the fingers reach the table, which is solid
//! ([`StepReject::PadsReachTable`]). Two ends finish the event: a pad that
//! slides past the end of a face, above the table, lets the object go
//! ([`StepReject::ObjectSlippedOut`]), and a lifted object that comes back
//! down onto the table, set down or dropped, has finished its lift
//! ([`StepReject::ObjectLanded`]).
//!
//! [`GraspLiftGlass4tu::gate_dt_s`] is the step ceiling: one twentieth of a
//! radian of the object's oscillation on both pads' springs, `sqrt(2K/m)`,
//! where the trapezoid rule's error of phase is about two parts in ten
//! thousand. [`GraspLiftGlass4tu::floor_dt_s`] is the ceiling divided by
//! 2^10. The public step applies neither. The gateway refuses a step outside
//! them.
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes time, the object's height, the
//! hand's height, the pads' travel, the two velocities, the stuck fraction,
//! the tangential force of one pad and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

/// The object's mass, kg. The company's own specimen.
pub const OBJECT_MASS_KG: f64 = 0.2;
/// Standard gravity, m/s².
pub const GRAVITY_M_S2: f64 = 9.80665;
/// Half the height of each glass face, m: the pads grip at the middle of
/// faces 60 mm tall. The company's own specimen.
pub const FACE_HALF_HEIGHT_M: f64 = 0.03;
/// The pads of the grip, one on each face.
pub const PAD_COUNT: f64 = 2.0;
/// Friction force of a pad over its grip force, static and kinetic alike.
/// Fitted to the development trials on natural glass (0.858686169).
pub const FRICTION_COEFFICIENT: f64 = 0.858686;
/// Tangential stiffness of one pad before any slip, N/m. Fitted to the
/// development trials on natural glass (339.696223).
pub const PAD_TANGENTIAL_STIFFNESS_N_M: f64 = 339.696;
/// `|defect| / scale` of either ledger must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;
/// The phase of the object's oscillation on the pads' springs over one step
/// at the step ceiling, radians.
pub const GATE_PHASE_STEP_RAD: f64 = 0.05;
/// Step ceiling divided by step floor: 2^10.
pub const GATE_OVER_FLOOR: f64 = 1024.0;
/// The slices of a pad's stuck centre, each one element.
pub const ELEMENTS: usize = 32;

/// The event of incipient slip: on this frame some of a pad's contact slid
/// and some held, the ring at its edge sliding and the centre stuck.
pub const FLAG_INCIPIENT_SLIP: u64 = 1 << 0;
/// The event of gross slip: on this frame the whole contact of each pad slid
/// along the faces.
pub const FLAG_GROSS_SLIP: u64 = 1 << 1;
/// A condition, not an event: the object rests on the table at the end of
/// this frame, not yet lifted.
pub const FLAG_ON_TABLE: u64 = 1 << 2;

/// Why a step did not advance the object or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The grip force, the hand's acceleration or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this specimen's.
    InvalidSpecimen,
    /// The grip force is zero or below: the pads do not press on the faces, and there is no contact to hold the object.
    NoContact,
    /// The object is below the surface of the table, which is solid.
    ObjectBelowTable,
    /// The pads slide down to the lower end of the faces of the object resting on the table: the fingers reach the table's surface, which is solid.
    PadsReachTable,
    /// A pad slides past the end of its face: the grip lets the object go, the end of the event.
    ObjectSlippedOut,
    /// The lifted object comes back down onto the table, set down or dropped: the end of its lift.
    ObjectLanded,
    /// The trial step's friction heat fell: friction gave energy back.
    FrictionHeatNegative,
    /// The trial step created energy: the object's energy, the springs' and the heat rose above the hand's work.
    SpontaneousGeneration,
    /// The trial step destroyed energy: the object's energy, the springs' and the heat fell below the hand's work.
    FirstLawBreach,
    /// The trial step's change of the object's momentum is not the impulse of the pads, gravity and the table.
    MomentumLedgerBroken,
}

/// What the hand holds over a step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraspLiftGlass4tuLoad {
    /// Grip force of each pad on its face, N.
    pub grip_force_n: f64,
    /// Vertical acceleration of the hand, m/s², upward positive.
    pub hand_acceleration_m_s2: f64,
}

/// The specimen. Every field is a recorded copy of a constant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraspLiftGlass4tu {
    pub object_mass_kg: f64,
    pub gravity_m_s2: f64,
    pub face_half_height_m: f64,
    pub pad_count: f64,
    pub friction_coefficient: f64,
    pub pad_tangential_stiffness_n_m: f64,
    pub residual_conservation_limit: f64,
    pub gate_phase_step_rad: f64,
    pub gate_over_floor: f64,
}

impl GraspLiftGlass4tu {
    /// The object in the grip. The constructor takes no arguments.
    #[must_use]
    pub fn specimen() -> Self {
        let card = Self {
            object_mass_kg: OBJECT_MASS_KG,
            gravity_m_s2: GRAVITY_M_S2,
            face_half_height_m: FACE_HALF_HEIGHT_M,
            pad_count: PAD_COUNT,
            friction_coefficient: FRICTION_COEFFICIENT,
            pad_tangential_stiffness_n_m: PAD_TANGENTIAL_STIFFNESS_N_M,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_phase_step_rad: GATE_PHASE_STEP_RAD,
            gate_over_floor: GATE_OVER_FLOOR,
        };
        if card.is_valid() {
            card
        } else {
            ::core::panic!("the specimen failed is_valid");
        }
    }

    /// Every recorded copy equals its constant, bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.object_mass_kg.to_bits() == OBJECT_MASS_KG.to_bits()
            && self.gravity_m_s2.to_bits() == GRAVITY_M_S2.to_bits()
            && self.face_half_height_m.to_bits() == FACE_HALF_HEIGHT_M.to_bits()
            && self.pad_count.to_bits() == PAD_COUNT.to_bits()
            && self.friction_coefficient.to_bits() == FRICTION_COEFFICIENT.to_bits()
            && self.pad_tangential_stiffness_n_m.to_bits() == PAD_TANGENTIAL_STIFFNESS_N_M.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.gate_phase_step_rad.to_bits() == GATE_PHASE_STEP_RAD.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
    }

    /// `F_s`, the friction force of one pad at this grip force, N. Zero for
    /// a grip of zero or below; [`step`] refuses those.
    #[must_use]
    pub fn friction_force_n(&self, grip_force_n: f64) -> f64 {
        if grip_force_n > 0.0 { self.friction_coefficient * grip_force_n } else { 0.0 }
    }

    /// The least grip force at which the friction of both pads together holds
    /// the object's weight at rest, N: `2 F_s(N) = m g`. A lift needs more.
    #[must_use]
    pub fn least_grip_at_rest_n(&self) -> f64 {
        self.object_mass_kg * self.gravity_m_s2 / (self.pad_count * self.friction_coefficient)
    }

    /// `sqrt(2K/m)`, the object's oscillation on both pads' springs, rad/s.
    #[must_use]
    pub fn contact_pole_rad_s(&self) -> f64 {
        (self.pad_count * self.pad_tangential_stiffness_n_m / self.object_mass_kg).sqrt()
    }

    /// The largest step the card stands behind: a twentieth of a radian of
    /// the oscillation on the pads' springs. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let omega = self.contact_pole_rad_s();
        if omega.is_finite() && omega > 0.0 {
            Some(self.gate_phase_step_rad / omega)
        } else {
            None
        }
    }

    /// The smallest step the card stands behind: the ceiling over 2^10. Not
    /// applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate) if self.gate_over_floor.is_finite() && self.gate_over_floor > 0.0 => {
                Some(gate / self.gate_over_floor)
            }
            _ => None,
        }
    }

    /// The floor of the energy ledger's scale: the object's potential energy
    /// over the height of its faces, `2 m g H`, J.
    #[must_use]
    pub fn energy_scale_floor_j(&self) -> f64 {
        2.0 * self.object_mass_kg * self.gravity_m_s2 * self.face_half_height_m
    }

    /// The floor of the momentum ledger's scale: the object's momentum after
    /// a fall from the height of its faces, `m sqrt(2 g · 2H)`, N·s.
    #[must_use]
    pub fn momentum_scale_floor_n_s(&self) -> f64 {
        self.object_mass_kg * (4.0 * self.gravity_m_s2 * self.face_half_height_m).sqrt()
    }

    /// Element `j`'s share of the pad's stiffness and of its friction force.
    #[must_use]
    pub fn element_shares(&self) -> ([f64; ELEMENTS], [f64; ELEMENTS]) {
        element_shares()
    }

    /// The stiffness of each element of one pad, N/m.
    #[must_use]
    pub fn element_stiffness_n_m(&self) -> [f64; ELEMENTS] {
        let (stiff, _) = element_shares();
        let mut out = [0.0; ELEMENTS];
        let mut j = 0;
        while j < ELEMENTS {
            out[j] = stiff[j] * self.pad_tangential_stiffness_n_m;
            j += 1;
        }
        out
    }

    /// The strength of each element of one pad at this grip force, N.
    #[must_use]
    pub fn element_strength_n(&self, grip_force_n: f64) -> [f64; ELEMENTS] {
        let (_, strength) = element_shares();
        let friction = self.friction_force_n(grip_force_n);
        let mut out = [0.0; ELEMENTS];
        let mut j = 0;
        while j < ELEMENTS {
            out[j] = strength[j] * friction;
            j += 1;
        }
        out
    }

    /// The tangential force of one pad on a first loading through the travel
    /// `d` at this grip force, N: the elements' sum, Cattaneo-Mindlin's law at
    /// the ends of the slices.
    #[must_use]
    pub fn first_loading_force_n(&self, grip_force_n: f64, travel_m: f64) -> f64 {
        let stiff = self.element_stiffness_n_m();
        let strength = self.element_strength_n(grip_force_n);
        let mut sum = 0.0;
        let mut j = 0;
        while j < ELEMENTS {
            let elastic = stiff[j] * travel_m.abs();
            sum += if elastic < strength[j] { elastic } else { strength[j] };
            j += 1;
        }
        if travel_m < 0.0 { -sum } else { sum }
    }
}

/// `u_j = sqrt(1 − j/n)`; the stiffness share `u_{j−1} − u_j` and the
/// strength share `1.5 [g(u_{j−1}) − g(u_j)]`, `g(u) = u − u³/3`, of each
/// element, the outermost slice first. Each set of shares sums to one.
fn element_shares() -> ([f64; ELEMENTS], [f64; ELEMENTS]) {
    let mut stiff = [0.0; ELEMENTS];
    let mut strength = [0.0; ELEMENTS];
    let g = |u: f64| u - u * u * u / 3.0;
    let mut outer = 1.0_f64;
    let mut j = 1;
    while j <= ELEMENTS {
        let inner = if j == ELEMENTS { 0.0 } else { (1.0 - (j as f64) / (ELEMENTS as f64)).sqrt() };
        stiff[j - 1] = outer - inner;
        strength[j - 1] = 1.5 * (g(outer) - g(inner));
        outer = inner;
        j += 1;
    }
    (stiff, strength)
}

/// The object, the hand and the two pads. The ledger fields are part of the
/// state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraspLiftGlass4tuState {
    /// Time since the grip closed, s.
    pub t_s: f64,
    /// Height of the hand above its height at the grip, m.
    pub hand_height_m: f64,
    /// Vertical velocity of the hand, m/s.
    pub hand_velocity_m_s: f64,
    /// Height of the object above the table, m. Zero while it rests on it.
    pub object_height_m: f64,
    /// Vertical velocity of the object, m/s.
    pub object_velocity_m_s: f64,
    /// The force of each element of one pad on the object, N, upward
    /// positive, the outermost slice first. Both pads are alike.
    pub element_force_n: [f64; ELEMENTS],
    /// The grip force of the last step, N. Zero before the first step.
    pub grip_force_n: f64,
    /// The hand's acceleration of the last step, m/s².
    pub hand_acceleration_m_s2: f64,
    /// Travel of the pads along the faces since the grip, `z_h − z`, m.
    pub pad_travel_m: f64,
    /// Tangential force of one pad on the object at the end of the step, N.
    pub tangential_force_n: f64,
    /// Friction force of one pad at the step's grip, N.
    pub friction_limit_n: f64,
    /// `|tangential force| / friction force`. One minus it is the safety margin.
    pub load_ratio: f64,
    /// Share of a pad's contact that did not slide in the step.
    pub stuck_fraction: f64,
    /// The object's mean acceleration over the step, m/s².
    pub object_acceleration_m_s2: f64,
    /// The table's mean push on the object over the step, N.
    pub table_force_n: f64,
    /// `½ m v²`, J.
    pub kinetic_energy_j: f64,
    /// `m g z`, J.
    pub potential_energy_j: f64,
    /// Energy in the springs of both pads, J.
    pub stored_energy_j: f64,
    /// The object's energy and the springs' at the start, J.
    pub energy_anchor_j: f64,
    /// Work of the hand through both pads since the start, `W_in − W_out`, J.
    pub hand_work_j: f64,
    /// Work the hand gave the object since the start, over the steps where
    /// its work was positive, J. Never falls.
    pub hand_work_in_j: f64,
    /// Work the object gave back to the hand since the start, over the
    /// steps where the hand's work was negative, J. Never falls.
    pub hand_work_out_j: f64,
    /// Heat of the sliding in both pads since the start, J. Never falls.
    pub friction_heat_j: f64,
    /// Signed `(K + U_g + E_springs) − E_anchor + H − W`. Positive is created energy.
    pub energy_defect_j: f64,
    /// What is in play in the energy ledger, `|E_anchor| + K + |U_g| +
    /// E_springs + H + W_in + W_out`, and never less than `2 m g H`, J.
    pub energy_scale_j: f64,
    /// `m v`, N·s.
    pub momentum_n_s: f64,
    /// `m v` at the start, N·s.
    pub momentum_anchor_n_s: f64,
    /// Impulse of both pads on the object since the start, N·s.
    pub contact_impulse_n_s: f64,
    /// Impulse of gravity since the start, N·s.
    pub gravity_impulse_n_s: f64,
    /// Impulse of the table since the start, N·s.
    pub table_impulse_n_s: f64,
    /// Signed `m v − p_anchor − (contact + gravity + table impulse)`.
    pub momentum_defect_n_s: f64,
    /// What is in play in the momentum ledger, `|m v| + |p_anchor| +
    /// |contact| + |gravity| + |table impulse|`, and never less than
    /// `m sqrt(4 g H)`, N·s.
    pub momentum_scale_n_s: f64,
    /// The larger of `|energy defect| / energy scale` and `|momentum defect| / momentum scale`.
    pub residual_conservation_error: f64,
    /// Distance the whole contact of a pad has slid along its face since the start, m.
    pub gross_slip_m: f64,
    /// The largest load ratio of any step since the start.
    pub peak_load_ratio: f64,
    /// The whole contact of the pads has slid in some step since the start:
    /// the object slipped in the grip.
    pub slipped: bool,
    /// Some of a pad's contact slid in the step and some held.
    pub incipient_slip: bool,
    /// The whole contact of each pad slid in the step.
    pub gross_slip: bool,
    /// The object rests on the table at the end of the step.
    pub on_table: bool,
}

impl GraspLiftGlass4tuState {
    /// The grip just closed: the object at rest on the table, the pads
    /// unstressed, the hand at the given vertical velocity, the ledgers empty.
    #[must_use]
    pub fn try_start(card: &GraspLiftGlass4tu, hand_velocity_m_s: f64) -> Option<Self> {
        if !card.is_valid() || !hand_velocity_m_s.is_finite() {
            return None;
        }
        Some(Self {
            t_s: 0.0,
            hand_height_m: 0.0,
            hand_velocity_m_s: hand_velocity_m_s + 0.0,
            object_height_m: 0.0,
            object_velocity_m_s: 0.0,
            element_force_n: [0.0; ELEMENTS],
            grip_force_n: 0.0,
            hand_acceleration_m_s2: 0.0,
            pad_travel_m: 0.0,
            tangential_force_n: 0.0,
            friction_limit_n: 0.0,
            load_ratio: 0.0,
            stuck_fraction: 1.0,
            object_acceleration_m_s2: 0.0,
            table_force_n: card.object_mass_kg * card.gravity_m_s2,
            kinetic_energy_j: 0.0,
            potential_energy_j: 0.0,
            stored_energy_j: 0.0,
            energy_anchor_j: 0.0,
            hand_work_j: 0.0,
            hand_work_in_j: 0.0,
            hand_work_out_j: 0.0,
            friction_heat_j: 0.0,
            energy_defect_j: 0.0,
            energy_scale_j: card.energy_scale_floor_j(),
            momentum_n_s: 0.0,
            momentum_anchor_n_s: 0.0,
            contact_impulse_n_s: 0.0,
            gravity_impulse_n_s: 0.0,
            table_impulse_n_s: 0.0,
            momentum_defect_n_s: 0.0,
            momentum_scale_n_s: card.momentum_scale_floor_n_s(),
            residual_conservation_error: 0.0,
            gross_slip_m: 0.0,
            peak_load_ratio: 0.0,
            slipped: false,
            incipient_slip: false,
            gross_slip: false,
            on_table: true,
        })
    }

    /// The object rests on the table: at its surface and still.
    #[must_use]
    pub fn rests_on_table(&self) -> bool {
        self.object_height_m == 0.0 && self.object_velocity_m_s == 0.0
    }
}

/// One pad's elements followed along the travel `Δ` of a step.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Path {
    /// The pad's mean force over the travel, N.
    mean_force_n: f64,
    /// The elements' forces at the end, N.
    element_force_n: [f64; ELEMENTS],
    /// The distance each element's slider slid, m.
    slide_m: [f64; ELEMENTS],
    /// Whether each element slid.
    slid: [bool; ELEMENTS],
    /// The elements that slid.
    sliding: usize,
    /// The distance the whole contact slid once every element was sliding, m.
    gross_slide_m: f64,
}

/// Each element followed exactly along the travel `Δ`: elastic until it
/// reaches its strength, then sliding at it.
fn follow(
    stiff: &[f64; ELEMENTS],
    strength: &[f64; ELEMENTS],
    force: &[f64; ELEMENTS],
    travel_m: f64,
) -> Path {
    let mut path = Path {
        mean_force_n: 0.0,
        element_force_n: *force,
        slide_m: [0.0; ELEMENTS],
        slid: [false; ELEMENTS],
        sliding: 0,
        gross_slide_m: 0.0,
    };
    let sign = if travel_m < 0.0 { -1.0 } else { 1.0 };
    let along = travel_m * sign;
    let mut mean = 0.0;
    let mut last_yield = 0.0_f64;
    let mut j = 0;
    while j < ELEMENTS {
        let q = force[j] * sign;
        let reach = (strength[j] - q) / stiff[j];
        if along <= reach {
            mean += q + 0.5 * stiff[j] * along;
            // At the strength to within the last place: the travel ends on
            // the element's yield, and it does not slide.
            let end = q + stiff[j] * along;
            path.element_force_n[j] = (if end > strength[j] { strength[j] } else { end }) * sign;
        } else {
            let reach = if reach > 0.0 { reach } else { 0.0 };
            mean += strength[j] - 0.5 * stiff[j] * reach * (reach / along);
            path.element_force_n[j] = strength[j] * sign;
            path.slide_m[j] = along - reach;
            path.slid[j] = true;
            path.sliding += 1;
            if reach > last_yield {
                last_yield = reach;
            }
        }
        j += 1;
    }
    if path.sliding == ELEMENTS {
        path.gross_slide_m = along - last_yield;
    }
    path.mean_force_n = mean * sign;
    path
}

/// The travel of the pads over a step on a free object: the root of
/// `Φ(Δ) = h (2 Q̄(Δ) − m g) − (2m/h)(Δz_h − Δ) + 2 m v`, which rises with
/// `Δ`. Between the travels at which elements reach their strength,
/// `Δ Φ(Δ)` is a quadratic, solved in closed form.
fn free_travel(
    card: &GraspLiftGlass4tu,
    stiff: &[f64; ELEMENTS],
    strength: &[f64; ELEMENTS],
    force: &[f64; ELEMENTS],
    hand_rise_m: f64,
    object_velocity_m_s: f64,
    dt_s: f64,
) -> f64 {
    let m = card.object_mass_kg;
    let pads = card.pad_count;
    let mut held = 0.0;
    let mut j = 0;
    while j < ELEMENTS {
        held += force[j];
        j += 1;
    }
    let phi_zero = dt_s * (pads * held - m * card.gravity_m_s2) - (2.0 * m / dt_s) * hand_rise_m
        + 2.0 * m * object_velocity_m_s;
    if phi_zero == 0.0 {
        return 0.0;
    }
    // Mirror a downward travel onto an upward one.
    let sign = if phi_zero < 0.0 { 1.0 } else { -1.0 };
    let gravity = card.gravity_m_s2 * sign;
    let rise = hand_rise_m * sign;
    let velocity = object_velocity_m_s * sign;
    let mut q = [0.0; ELEMENTS];
    let mut reach = [0.0; ELEMENTS];
    let mut order = [0usize; ELEMENTS];
    j = 0;
    while j < ELEMENTS {
        q[j] = force[j] * sign;
        let r = (strength[j] - q[j]) / stiff[j];
        reach[j] = if r > 0.0 { r } else { 0.0 };
        order[j] = j;
        j += 1;
    }
    // The yield travels in rising order.
    let mut a = 1;
    while a < ELEMENTS {
        let mut b = a;
        while b > 0 && reach[order[b - 1]] > reach[order[b]] {
            order.swap(b - 1, b);
            b -= 1;
        }
        a += 1;
    }
    let phi = |travel: f64| -> f64 {
        let mut mean = 0.0;
        let mut k = 0;
        while k < ELEMENTS {
            if travel <= reach[k] {
                mean += q[k] + 0.5 * stiff[k] * travel;
            } else {
                mean += strength[k] - 0.5 * stiff[k] * reach[k] * (reach[k] / travel);
            }
            k += 1;
        }
        dt_s * (pads * mean - m * gravity) - (2.0 * m / dt_s) * (rise - travel) + 2.0 * m * velocity
    };
    // The piece that holds the root: below the first yield travel at which
    // Φ is not negative.
    let mut piece = ELEMENTS;
    let mut k = 0;
    while k < ELEMENTS {
        let at = reach[order[k]];
        if at > 0.0 && phi(at) >= 0.0 {
            piece = k;
            break;
        }
        k += 1;
    }
    // On that piece the elements before it in order slide and the rest are
    // elastic: W(Δ) = A + B Δ + C Δ².
    let mut big_a = 0.0;
    let mut big_b = 0.0;
    let mut big_c = 0.0;
    k = 0;
    while k < ELEMENTS {
        let e = order[k];
        if k < piece {
            big_a -= 0.5 * stiff[e] * reach[e] * reach[e];
            big_b += strength[e];
        } else {
            big_b += q[e];
            big_c += 0.5 * stiff[e];
        }
        k += 1;
    }
    let alpha = dt_s * pads * big_c + 2.0 * m / dt_s;
    let beta = dt_s * pads * big_b - dt_s * m * gravity - (2.0 * m / dt_s) * rise + 2.0 * m * velocity;
    let gamma = dt_s * pads * big_a;
    let travel = if gamma == 0.0 {
        -beta / alpha
    } else {
        let root = (beta * beta - 4.0 * alpha * gamma).sqrt();
        if beta >= 0.0 { -2.0 * gamma / (beta + root) } else { (root - beta) / (2.0 * alpha) }
    };
    travel * sign
}

/// What one step adds, each term worked out on its own.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Advance {
    hand_rise_m: f64,
    hand_velocity_m_s: f64,
    object_height_m: f64,
    object_velocity_m_s: f64,
    element_force_n: [f64; ELEMENTS],
    /// Mean force of both pads over the step, N.
    pads_mean_force_n: f64,
    /// Mean push of the table over the step, N.
    table_force_n: f64,
    /// Heat of both pads in the step, J.
    heat_j: f64,
    sliding: usize,
    gross_slide_m: f64,
}

/// Hold the load for one step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    card: &GraspLiftGlass4tu,
    state: &mut GraspLiftGlass4tuState,
    frame: &mut LastStateFrame64,
    load: GraspLiftGlass4tuLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(card, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    card: &GraspLiftGlass4tu,
    state: &mut GraspLiftGlass4tuState,
    frame: &mut LastStateFrame64,
    load: GraspLiftGlass4tuLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(card, state, load, dt_s)?;
    let advance = advance(card, state, load, dt_s);
    commit(card, state, frame, load, dt_s, advance)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut GraspLiftGlass4tuState) {
    state.t_s = normal_or_zero(state.t_s);
    state.hand_height_m = normal_or_zero(state.hand_height_m);
    state.hand_velocity_m_s = normal_or_zero(state.hand_velocity_m_s);
    state.object_height_m = normal_or_zero(state.object_height_m);
    state.object_velocity_m_s = normal_or_zero(state.object_velocity_m_s);
    normal_or_zero_each(&mut state.element_force_n);
    state.grip_force_n = normal_or_zero(state.grip_force_n);
    state.hand_acceleration_m_s2 = normal_or_zero(state.hand_acceleration_m_s2);
    state.pad_travel_m = normal_or_zero(state.pad_travel_m);
    state.tangential_force_n = normal_or_zero(state.tangential_force_n);
    state.friction_limit_n = normal_or_zero(state.friction_limit_n);
    state.load_ratio = normal_or_zero(state.load_ratio);
    state.stuck_fraction = normal_or_zero(state.stuck_fraction);
    state.object_acceleration_m_s2 = normal_or_zero(state.object_acceleration_m_s2);
    state.table_force_n = normal_or_zero(state.table_force_n);
    state.kinetic_energy_j = normal_or_zero(state.kinetic_energy_j);
    state.potential_energy_j = normal_or_zero(state.potential_energy_j);
    state.stored_energy_j = normal_or_zero(state.stored_energy_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.hand_work_j = normal_or_zero(state.hand_work_j);
    state.hand_work_in_j = normal_or_zero(state.hand_work_in_j);
    state.hand_work_out_j = normal_or_zero(state.hand_work_out_j);
    state.friction_heat_j = normal_or_zero(state.friction_heat_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.energy_scale_j = normal_or_zero(state.energy_scale_j);
    state.momentum_n_s = normal_or_zero(state.momentum_n_s);
    state.momentum_anchor_n_s = normal_or_zero(state.momentum_anchor_n_s);
    state.contact_impulse_n_s = normal_or_zero(state.contact_impulse_n_s);
    state.gravity_impulse_n_s = normal_or_zero(state.gravity_impulse_n_s);
    state.table_impulse_n_s = normal_or_zero(state.table_impulse_n_s);
    state.momentum_defect_n_s = normal_or_zero(state.momentum_defect_n_s);
    state.momentum_scale_n_s = normal_or_zero(state.momentum_scale_n_s);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.gross_slip_m = normal_or_zero(state.gross_slip_m);
    state.peak_load_ratio = normal_or_zero(state.peak_load_ratio);
}

fn validate(
    card: &GraspLiftGlass4tu,
    state: &GraspLiftGlass4tuState,
    load: GraspLiftGlass4tuLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !(dt_s.is_finite() && load.grip_force_n.is_finite() && load.hand_acceleration_m_s2.is_finite()) {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !card.is_valid() {
        return Err(StepReject::InvalidSpecimen);
    }
    if !(load.grip_force_n > 0.0) || !(card.friction_force_n(load.grip_force_n) > 0.0) {
        return Err(StepReject::NoContact);
    }
    if state.object_height_m < 0.0 {
        return Err(StepReject::ObjectBelowTable);
    }
    off_the_faces(card, state.hand_height_m - state.object_height_m, state.object_height_m)
}

/// Where the pads stand against the faces, at the travel `r = z_h − z` with
/// the object at height `z`: on the faces, at the table (the lower end of
/// the faces of an object at the table's surface), or past an end of a face
/// above the table, where the grip lets the object go.
fn off_the_faces(card: &GraspLiftGlass4tu, pad_travel_m: f64, object_height_m: f64) -> Result<(), StepReject> {
    if pad_travel_m.abs() < card.face_half_height_m {
        Ok(())
    } else if pad_travel_m < 0.0 && object_height_m == 0.0 {
        Err(StepReject::PadsReachTable)
    } else {
        Err(StepReject::ObjectSlippedOut)
    }
}

fn advance(card: &GraspLiftGlass4tu, state: &GraspLiftGlass4tuState, load: GraspLiftGlass4tuLoad, dt_s: f64) -> Advance {
    let stiff = card.element_stiffness_n_m();
    let strength = card.element_strength_n(load.grip_force_n);
    // A grip that fell: each element above its new strength slides back to
    // it at once, and the energy its spring gives up is heat.
    let mut force = state.element_force_n;
    let mut release_j = 0.0;
    let mut released = [false; ELEMENTS];
    let mut j = 0;
    while j < ELEMENTS {
        let q = force[j];
        if q > strength[j] || q < -strength[j] {
            let limit = if q > 0.0 { strength[j] } else { -strength[j] };
            release_j += (q - limit) * (q + limit) / (2.0 * stiff[j]);
            force[j] = limit;
            released[j] = true;
        }
        j += 1;
    }
    let h = dt_s;
    let a = load.hand_acceleration_m_s2;
    let hand_rise_m = state.hand_velocity_m_s * h + 0.5 * a * h * h;
    let hand_velocity_m_s = state.hand_velocity_m_s + a * h;
    let m = card.object_mass_kg;
    let weight = m * card.gravity_m_s2;
    let mut object_height_m = state.object_height_m;
    let mut object_velocity_m_s = state.object_velocity_m_s;
    let mut table_force_n = 0.0;
    let mut path = follow(&stiff, &strength, &force, hand_rise_m);
    let mut pads = card.pad_count * path.mean_force_n;
    if !(state.rests_on_table() && pads <= weight) {
        let travel = free_travel(card, &stiff, &strength, &force, hand_rise_m, state.object_velocity_m_s, h);
        path = follow(&stiff, &strength, &force, travel);
        pads = card.pad_count * path.mean_force_n;
        object_velocity_m_s = (2.0 / h) * (hand_rise_m - travel) - state.object_velocity_m_s;
        object_height_m = state.object_height_m + 0.5 * h * (state.object_velocity_m_s + object_velocity_m_s);
    } else {
        table_force_n = weight - pads;
    }
    let mut slide_heat_j = 0.0;
    let mut sliding = 0;
    j = 0;
    while j < ELEMENTS {
        slide_heat_j += strength[j] * path.slide_m[j];
        if released[j] || path.slid[j] {
            sliding += 1;
        }
        j += 1;
    }
    Advance {
        hand_rise_m,
        hand_velocity_m_s,
        object_height_m,
        object_velocity_m_s,
        element_force_n: path.element_force_n,
        pads_mean_force_n: pads,
        table_force_n,
        heat_j: card.pad_count * (release_j + slide_heat_j),
        sliding,
        gross_slide_m: path.gross_slide_m,
    }
}

fn commit(
    card: &GraspLiftGlass4tu,
    state: &mut GraspLiftGlass4tuState,
    frame: &mut LastStateFrame64,
    load: GraspLiftGlass4tuLoad,
    dt_s: f64,
    advance: Advance,
) -> Result<(), StepReject> {
    if !(advance.heat_j >= 0.0) {
        return if advance.heat_j.is_finite() { Err(StepReject::FrictionHeatNegative) } else { Err(StepReject::NonFiniteState) };
    }
    let mut next = *state;
    let m = card.object_mass_kg;
    let weight = m * card.gravity_m_s2;
    next.t_s += dt_s;
    next.hand_height_m += advance.hand_rise_m;
    next.hand_velocity_m_s = advance.hand_velocity_m_s;
    next.object_height_m = advance.object_height_m;
    next.object_velocity_m_s = advance.object_velocity_m_s;
    next.element_force_n = advance.element_force_n;
    next.grip_force_n = load.grip_force_n;
    next.hand_acceleration_m_s2 = load.hand_acceleration_m_s2;
    let work_j = advance.pads_mean_force_n * advance.hand_rise_m;
    if work_j >= 0.0 {
        next.hand_work_in_j += work_j;
    } else {
        next.hand_work_out_j -= work_j;
    }
    next.hand_work_j = next.hand_work_in_j - next.hand_work_out_j;
    next.friction_heat_j += advance.heat_j;
    next.contact_impulse_n_s += dt_s * advance.pads_mean_force_n;
    next.gravity_impulse_n_s -= dt_s * weight;
    next.table_impulse_n_s += dt_s * advance.table_force_n;
    next.table_force_n = advance.table_force_n;
    next.object_acceleration_m_s2 = (advance.object_velocity_m_s - state.object_velocity_m_s) / dt_s;
    next.gross_slip_m += advance.gross_slide_m;
    next.stuck_fraction = 1.0 - (advance.sliding as f64) / (ELEMENTS as f64);
    next.incipient_slip = advance.sliding > 0 && advance.sliding < ELEMENTS;
    next.gross_slip = advance.sliding == ELEMENTS;
    next.slipped = state.slipped || next.gross_slip;
    refresh(card, &mut next);
    next.on_table = next.rests_on_table();
    next.energy_defect_j = (next.kinetic_energy_j + next.potential_energy_j + next.stored_energy_j)
        - next.energy_anchor_j
        + next.friction_heat_j
        - next.hand_work_j;
    next.momentum_defect_n_s = next.momentum_n_s
        - next.momentum_anchor_n_s
        - (next.contact_impulse_n_s + next.gravity_impulse_n_s + next.table_impulse_n_s);
    // What is in play in each ledger: the size of each of its terms.
    let energy_in_play = next.energy_anchor_j.abs()
        + next.kinetic_energy_j
        + next.potential_energy_j.abs()
        + next.stored_energy_j
        + next.friction_heat_j
        + next.hand_work_in_j
        + next.hand_work_out_j;
    let momentum_in_play = next.momentum_n_s.abs()
        + next.momentum_anchor_n_s.abs()
        + next.contact_impulse_n_s.abs()
        + next.gravity_impulse_n_s.abs()
        + next.table_impulse_n_s.abs();
    let energy_floor = card.energy_scale_floor_j();
    let momentum_floor = card.momentum_scale_floor_n_s();
    next.energy_scale_j = if energy_in_play > energy_floor { energy_in_play } else { energy_floor };
    next.momentum_scale_n_s = if momentum_in_play > momentum_floor { momentum_in_play } else { momentum_floor };
    let energy_residual = next.energy_defect_j.abs() / next.energy_scale_j;
    let momentum_residual = next.momentum_defect_n_s.abs() / next.momentum_scale_n_s;
    next.residual_conservation_error =
        if energy_residual > momentum_residual { energy_residual } else { momentum_residual };
    if next.load_ratio > next.peak_load_ratio {
        next.peak_load_ratio = next.load_ratio;
    }
    if !state_is_finite(&next) {
        return Err(StepReject::NonFiniteState);
    }
    if next.energy_defect_j > card.residual_conservation_limit * next.energy_scale_j {
        return Err(StepReject::SpontaneousGeneration);
    }
    if next.energy_defect_j < -card.residual_conservation_limit * next.energy_scale_j {
        return Err(StepReject::FirstLawBreach);
    }
    if !(momentum_residual <= card.residual_conservation_limit) {
        return Err(StepReject::MomentumLedgerBroken);
    }
    if next.object_height_m < 0.0 {
        return Err(StepReject::ObjectLanded);
    }
    off_the_faces(card, next.pad_travel_m, next.object_height_m)?;
    *state = next;
    publish(state, frame);
    Ok(())
}

/// The quantities of a state that follow from its positions, velocities
/// and element forces.
fn refresh(card: &GraspLiftGlass4tu, state: &mut GraspLiftGlass4tuState) {
    let stiff = card.element_stiffness_n_m();
    let mut force = 0.0;
    let mut stored = 0.0;
    let mut j = 0;
    while j < ELEMENTS {
        let q = state.element_force_n[j];
        force += q;
        stored += q * q / (2.0 * stiff[j]);
        j += 1;
    }
    let m = card.object_mass_kg;
    state.pad_travel_m = state.hand_height_m - state.object_height_m;
    state.tangential_force_n = force;
    state.friction_limit_n = card.friction_force_n(state.grip_force_n);
    state.load_ratio = if state.friction_limit_n > 0.0 { force.abs() / state.friction_limit_n } else { 0.0 };
    state.stored_energy_j = card.pad_count * stored;
    state.kinetic_energy_j = 0.5 * m * state.object_velocity_m_s * state.object_velocity_m_s;
    state.potential_energy_j = m * card.gravity_m_s2 * state.object_height_m;
    state.momentum_n_s = m * state.object_velocity_m_s;
}

fn state_is_finite(state: &GraspLiftGlass4tuState) -> bool {
    let mut j = 0;
    while j < ELEMENTS {
        if !state.element_force_n[j].is_finite() {
            return false;
        }
        j += 1;
    }
    state.t_s.is_finite()
        && state.hand_height_m.is_finite()
        && state.hand_velocity_m_s.is_finite()
        && state.object_height_m.is_finite()
        && state.object_velocity_m_s.is_finite()
        && state.grip_force_n.is_finite()
        && state.hand_acceleration_m_s2.is_finite()
        && state.pad_travel_m.is_finite()
        && state.tangential_force_n.is_finite()
        && state.friction_limit_n.is_finite()
        && state.load_ratio.is_finite()
        && state.stuck_fraction.is_finite()
        && state.object_acceleration_m_s2.is_finite()
        && state.table_force_n.is_finite()
        && state.kinetic_energy_j.is_finite()
        && state.potential_energy_j.is_finite()
        && state.stored_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.hand_work_j.is_finite()
        && state.hand_work_in_j.is_finite()
        && state.hand_work_out_j.is_finite()
        && state.friction_heat_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.energy_scale_j.is_finite()
        && state.momentum_scale_n_s.is_finite()
        && state.momentum_n_s.is_finite()
        && state.momentum_anchor_n_s.is_finite()
        && state.contact_impulse_n_s.is_finite()
        && state.gravity_impulse_n_s.is_finite()
        && state.table_impulse_n_s.is_finite()
        && state.momentum_defect_n_s.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.gross_slip_m.is_finite()
        && state.peak_load_ratio.is_finite()
}

fn publish(state: &GraspLiftGlass4tuState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.object_height_m as f32;
    frame.pos[1] = state.hand_height_m as f32;
    frame.pos[2] = state.pad_travel_m as f32;
    frame.vel[0] = state.object_velocity_m_s as f32;
    frame.vel[1] = state.hand_velocity_m_s as f32;
    frame.vel[2] = state.stuck_fraction as f32;
    frame.force_torque = state.tangential_force_n as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.incipient_slip {
        flags |= FLAG_INCIPIENT_SLIP;
    }
    if state.gross_slip {
        flags |= FLAG_GROSS_SLIP;
    }
    if state.on_table {
        flags |= FLAG_ON_TABLE;
    }
    frame.flags = flags;
}

/// One step by forward Euler: the object's velocity from the pads' force at
/// the start of the step, booked with the honest ledger terms. A wrong
/// stepping method, for the tests.
#[cfg(test)]
fn step_euler(
    card: &GraspLiftGlass4tu,
    state: &mut GraspLiftGlass4tuState,
    frame: &mut LastStateFrame64,
    load: GraspLiftGlass4tuLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(card, state, load, dt_s)?;
    let mut honest = advance(card, state, load, dt_s);
    let mut held = 0.0;
    let mut j = 0;
    while j < ELEMENTS {
        held += state.element_force_n[j];
        j += 1;
    }
    let force = card.pad_count * held - card.object_mass_kg * card.gravity_m_s2;
    honest.object_velocity_m_s = state.object_velocity_m_s + dt_s * force / card.object_mass_kg;
    honest.object_height_m = state.object_height_m + dt_s * state.object_velocity_m_s;
    commit(card, state, frame, load, dt_s, honest)
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::matches;
    use std::vec::Vec;

    use super::*;

    const PROOF: [u8; 16] = [0xA5; 16];

    fn blank() -> LastStateFrame64 {
        LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: PROOF }
    }

    fn card() -> GraspLiftGlass4tu {
        GraspLiftGlass4tu::specimen()
    }

    fn gate() -> f64 {
        card().gate_dt_s().unwrap()
    }

    fn load(grip_force_n: f64, hand_acceleration_m_s2: f64) -> GraspLiftGlass4tuLoad {
        GraspLiftGlass4tuLoad { grip_force_n, hand_acceleration_m_s2 }
    }

    /// Run segments of (steps, load) from a fresh grip; stop at the first refusal.
    fn run(segments: &[(usize, GraspLiftGlass4tuLoad)], dt_s: f64) -> (Vec<GraspLiftGlass4tuState>, Option<StepReject>) {
        let card = card();
        let mut state = GraspLiftGlass4tuState::try_start(&card, 0.0).unwrap();
        let mut frame = blank();
        let mut out = Vec::new();
        for &(steps, held) in segments {
            for _ in 0..steps {
                let before = state;
                match step(&card, &mut state, &mut frame, held, dt_s) {
                    Ok(()) => out.push(state),
                    Err(reject) => {
                        assert_eq!(state, before);
                        return (out, Some(reject));
                    }
                }
                assert_eq!(frame.proof, PROOF);
            }
        }
        (out, None)
    }

    /// The lift of the worked example: 2 m/s² for 0.3 s, then the hand's
    /// speed held for 0.2 s.
    fn lift(grip_force_n: f64, dt_s: f64) -> (Vec<GraspLiftGlass4tuState>, Option<StepReject>) {
        let accelerate = (0.3 / dt_s).round() as usize;
        let hold = (0.2 / dt_s).round() as usize;
        run(&[(accelerate, load(grip_force_n, 2.0)), (hold, load(grip_force_n, 0.0))], dt_s)
    }

    #[test]
    fn the_specimen_holds_its_constants_and_a_moved_one_is_refused() {
        let card = card();
        assert!(card.is_valid());
        let mut moved = card;
        moved.pad_tangential_stiffness_n_m = f64::from_bits(PAD_TANGENTIAL_STIFFNESS_N_M.to_bits() + 1);
        assert!(!moved.is_valid());
        let mut state = GraspLiftGlass4tuState::try_start(&card, 0.0).unwrap();
        let before = state;
        let mut frame = blank();
        assert_eq!(step(&moved, &mut state, &mut frame, load(1.0, 1.0), gate()), Err(StepReject::InvalidSpecimen));
        assert_eq!(state, before);
        assert!(GraspLiftGlass4tuState::try_start(&moved, 0.0).is_none());
    }

    /// The shares sum to one; on a first loading the elements give
    /// Cattaneo-Mindlin's force at the end of every slice, and the share that
    /// has not slid is its stuck fraction.
    #[test]
    fn the_elements_are_cattaneo_mindlin_at_the_ends_of_the_slices() {
        let (stiff, strength) = element_shares();
        let sum = |x: &[f64; ELEMENTS]| x.iter().sum::<f64>();
        assert!((sum(&stiff) - 1.0).abs() < 1.0e-15);
        assert!((sum(&strength) - 1.0).abs() < 1.0e-15);
        let card = card();
        for grip in [0.3, 1.0, 2.0, 5.0] {
            let friction = card.friction_force_n(grip);
            let travel_full = 1.5 * friction / PAD_TANGENTIAL_STIFFNESS_N_M;
            for j in 0..=ELEMENTS {
                let x = (j as f64) / (ELEMENTS as f64);
                let law = friction * (1.0 - (1.0 - x).powf(1.5));
                let got = card.first_loading_force_n(grip, x * travel_full);
                assert!((got - law).abs() <= 1.0e-12 * friction, "{grip} {j} {got} {law}");
                // The stuck fraction at the slice end is (1 − Q/F)^(2/3).
                let stuck = 1.0 - x;
                assert!(((1.0 - law / friction).powf(2.0 / 3.0) - stuck).abs() < 1.0e-12);
            }
        }
    }

    /// The friction law and the least grip at rest.
    #[test]
    fn the_least_grip_at_rest_holds_the_weight_exactly() {
        let card = card();
        let least = card.least_grip_at_rest_n();
        let held = card.pad_count * card.friction_force_n(least);
        assert!((held / (OBJECT_MASS_KG * GRAVITY_M_S2) - 1.0).abs() < 1.0e-14);
        assert_eq!(card.friction_force_n(0.0), 0.0);
        assert_eq!(card.friction_force_n(-1.0), 0.0);
        assert_eq!(card.friction_force_n(1.0), FRICTION_COEFFICIENT);
        assert!((least - 1.1420531).abs() < 1.0e-6, "{least}");
    }

    /// A lift at an ample grip: the object leaves the table, follows the
    /// hand, never slides as a whole, and both ledgers close at every step,
    /// at the ceiling and at a quarter of it.
    #[test]
    fn an_ample_grip_lifts_without_gross_slip_and_both_ledgers_close() {
        for dt_s in [gate(), gate() / 4.0] {
            let (states, stop) = lift(4.0, dt_s);
            assert_eq!(stop, None);
            let last = states.last().unwrap();
            assert!(last.object_height_m > 0.1, "{}", last.object_height_m);
            assert!(!last.on_table);
            assert_eq!(last.gross_slip_m, 0.0);
            assert!(last.peak_load_ratio < 1.0);
            assert!(states.iter().all(|s| s.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT / 100.0));
            assert!(states.iter().all(|s| !s.gross_slip));
            // The object swings on the pads' springs about the hand's speed,
            // 0.6 m/s: over the last tenth of a second it is near it.
            let tail = (0.1 / dt_s).round() as usize;
            let mean = states[states.len() - tail..].iter().map(|s| s.object_velocity_m_s).sum::<f64>() / (tail as f64);
            assert!((mean - 0.6).abs() < 0.03, "{mean}");
            // The heat never falls.
            assert!(states.windows(2).all(|w| w[1].friction_heat_j >= w[0].friction_heat_j));
        }
    }

    /// The object rests on the table until the pads' force passes its
    /// weight, and the table carries the rest.
    #[test]
    fn the_object_rests_until_the_pads_carry_its_weight() {
        let (states, _) = lift(4.0, gate());
        let weight = OBJECT_MASS_KG * GRAVITY_M_S2;
        let first_lifted = states.iter().position(|s| !s.on_table).unwrap();
        assert!(first_lifted > 0);
        for s in &states[..first_lifted] {
            assert_eq!(s.object_height_m, 0.0);
            assert!(s.table_force_n >= 0.0);
            assert!((s.table_force_n + PAD_COUNT * s.tangential_force_n - weight).abs() < 0.02 * weight);
        }
        assert!(PAD_COUNT * states[first_lifted - 1].tangential_force_n <= weight * 1.02);
    }

    /// Under a grip whose friction cannot hold the weight, the object never
    /// leaves the table: the pads slide up its faces until they pass their
    /// ends, the end of the event, with the state whole.
    #[test]
    fn a_grip_under_the_weight_slides_out_and_ends() {
        let card = card();
        let weak = 0.5 * card.least_grip_at_rest_n();
        let (states, stop) = run(&[(100_000, load(weak, 2.0))], gate());
        assert_eq!(stop, Some(StepReject::ObjectSlippedOut));
        let last = states.last().unwrap();
        assert!(states.iter().all(|s| s.on_table));
        assert!(last.gross_slip && last.gross_slip_m > 0.0);
        assert!(last.pad_travel_m < FACE_HALF_HEIGHT_M && last.pad_travel_m > 0.9 * FACE_HALF_HEIGHT_M);
        assert!(last.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    /// Lifted, then lowered back onto the table: the lift ends where the
    /// object touches down.
    #[test]
    fn a_lifted_object_set_down_ends_its_lift() {
        let dt = gate();
        let up = (0.2 / dt).round() as usize;
        let (states, stop) = run(
            &[(up, load(3.0, 1.0)), (2 * up, load(3.0, -1.0)), (100_000, load(3.0, 0.0))],
            dt,
        );
        assert_eq!(stop, Some(StepReject::ObjectLanded));
        let last = states.last().unwrap();
        assert!(last.object_height_m >= 0.0 && last.object_velocity_m_s < 0.0);
        assert!(states.iter().any(|s| s.object_height_m > 0.01));
    }

    /// No grip is no contact; an object under the table's surface is no
    /// state; and nothing moves.
    #[test]
    fn each_physical_limit_is_named_and_nothing_moves() {
        let card = card();
        let (states, _) = lift(4.0, gate());
        let start = states[300];
        let mut start_frame = blank();
        publish(&start, &mut start_frame);
        let good = load(4.0, 0.0);
        let cases = [
            (load(0.0, 0.0), gate(), StepReject::NoContact),
            (load(-1.0, 0.0), gate(), StepReject::NoContact),
            (load(-0.0, 0.0), gate(), StepReject::NoContact),
            (load(f64::NAN, 0.0), gate(), StepReject::NonFiniteInput),
            (load(4.0, f64::INFINITY), gate(), StepReject::NonFiniteInput),
            (good, f64::NAN, StepReject::NonFiniteInput),
            (good, 0.0, StepReject::NonPositiveDt),
            (good, -gate(), StepReject::NonPositiveDt),
        ];
        for (held, dt_s, want) in cases {
            let mut state = start;
            let mut frame = start_frame;
            assert_eq!(step(&card, &mut state, &mut frame, held, dt_s), Err(want));
            assert_eq!(state, start);
            assert_eq!(frame.to_bytes(), start_frame.to_bytes());
        }
        let mut below = start;
        below.object_height_m = -1.0e-9;
        let saved = below;
        let mut frame = blank();
        assert_eq!(step(&card, &mut below, &mut frame, good, gate()), Err(StepReject::ObjectBelowTable));
        assert_eq!(below, saved);
        let mut broken = start;
        broken.element_force_n[7] = f64::NAN;
        let bits = broken.element_force_n[7].to_bits();
        assert_eq!(step(&card, &mut broken, &mut frame, good, gate()), Err(StepReject::NonFiniteState));
        assert_eq!(broken.element_force_n[7].to_bits(), bits);
        assert!(GraspLiftGlass4tuState::try_start(&card, f64::NAN).is_none());
    }

    /// Rule 10. Forward Euler is named by the ledgers; a falsified ledger is
    /// named; a step whose heat falls is named; nothing moves.
    #[test]
    fn a_wrong_stepping_method_and_a_falsified_ledger_are_named() {
        let card = card();
        let (states, _) = lift(4.0, gate());
        let mut state = states[400];
        assert!(!state.on_table);
        let mut frame = blank();
        let mut named = None;
        for _ in 0..50 {
            let saved = state;
            match step_euler(&card, &mut state, &mut frame, load(4.0, 0.0), gate()) {
                Ok(()) => {}
                Err(reject) => {
                    assert_eq!(state, saved);
                    named = Some(reject);
                    break;
                }
            }
        }
        assert!(matches!(named, Some(StepReject::SpontaneousGeneration | StepReject::FirstLawBreach | StepReject::MomentumLedgerBroken)), "{named:?}");

        let start = states[400];
        for (factor, want) in [(1.0 + 1.0e-3, StepReject::SpontaneousGeneration), (1.0 - 1.0e-3, StepReject::FirstLawBreach)] {
            let mut forged = advance(&card, &start, load(4.0, 0.0), gate());
            forged.heat_j = forged.heat_j * factor + (factor - 1.0) * 1.0e-6;
            let mut trial = start;
            let mut trial_frame = blank();
            assert_eq!(commit(&card, &mut trial, &mut trial_frame, load(4.0, 0.0), gate(), forged), Err(want));
            assert_eq!(trial, start);
        }
        let mut forged = advance(&card, &start, load(4.0, 0.0), gate());
        forged.heat_j = -1.0e-12;
        let mut trial = start;
        let mut trial_frame = blank();
        assert_eq!(commit(&card, &mut trial, &mut trial_frame, load(4.0, 0.0), gate(), forged), Err(StepReject::FrictionHeatNegative));
        let mut forged = advance(&card, &start, load(4.0, 0.0), gate());
        forged.object_velocity_m_s += 1.0e-6;
        forged.object_height_m += 0.5 * gate() * 1.0e-6;
        let mut trial = start;
        assert!(matches!(
            commit(&card, &mut trial, &mut trial_frame, load(4.0, 0.0), gate(), forged),
            Err(StepReject::MomentumLedgerBroken | StepReject::SpontaneousGeneration)
        ));
        assert_eq!(trial, start);
        let mut falsified = start;
        falsified.energy_anchor_j -= 1.0e-6;
        assert_eq!(step(&card, &mut falsified, &mut trial_frame, load(4.0, 0.0), gate()), Err(StepReject::SpontaneousGeneration));
        let mut falsified = start;
        falsified.contact_impulse_n_s += 1.0e-6;
        assert_eq!(step(&card, &mut falsified, &mut trial_frame, load(4.0, 0.0), gate()), Err(StepReject::MomentumLedgerBroken));
    }

    /// A grip relaxed while the object hangs: the edge of each contact slides
    /// back to its new strength at once, its spring's energy goes to heat,
    /// and the ledgers close.
    #[test]
    fn a_relaxed_grip_slides_the_edge_and_books_the_heat() {
        let card = card();
        let dt = gate();
        let (states, _) = run(&[(400, load(4.0, 1.0)), (400, load(4.0, 0.0))], dt);
        let hanging = *states.last().unwrap();
        let mut state = hanging;
        let mut frame = blank();
        // Released at once, the object drops onto the softer contact and swings
        // past its new rest: a grip of twice the least at rest holds it.
        let relaxed = 2.0 * card.least_grip_at_rest_n();
        step(&card, &mut state, &mut frame, load(relaxed, 0.0), dt).unwrap();
        assert!(state.friction_heat_j > hanging.friction_heat_j);
        assert!(state.incipient_slip);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT / 100.0);
        // The object swings on the softer contact; over the last of 4,000
        // steps its pads carry its weight on average.
        let mut mean = 0.0;
        for index in 0..4000 {
            step(&card, &mut state, &mut frame, load(relaxed, 0.0), dt).unwrap();
            if index >= 3000 {
                mean += PAD_COUNT * state.tangential_force_n / 1000.0;
            }
        }
        assert!(state.gross_slip_m == 0.0, "{}", state.gross_slip_m);
        assert!((mean / (OBJECT_MASS_KG * GRAVITY_M_S2) - 1.0).abs() < 0.02, "{mean}");
    }

    /// The 4TU set's own motion: the hand slides down along the object as it
    /// rests on the table, at 10 mm/s and a held grip. The pads load to their
    /// friction force, the stuck share falls as Cattaneo-Mindlin's
    /// (1 − Q/F)^(2/3) to within one slice, and the whole contact slides.
    #[test]
    fn the_plate_motion_of_the_4tu_set_follows_the_law() {
        let card = card();
        let dt = gate();
        let mut state = GraspLiftGlass4tuState::try_start(&card, -0.01).unwrap();
        let mut frame = blank();
        let grip = 1.2;
        let friction = card.friction_force_n(grip);
        let mut seen_gross = false;
        for _ in 0..((0.8 / dt) as usize) {
            step(&card, &mut state, &mut frame, load(grip, 0.0), dt).unwrap();
            assert!(state.on_table);
            let ratio = state.load_ratio;
            let law = if ratio < 1.0 { (1.0 - ratio).powf(2.0 / 3.0) } else { 0.0 };
            assert!((state.stuck_fraction - law).abs() <= 1.0 / (ELEMENTS as f64) + 1.0e-12, "{} {}", state.stuck_fraction, law);
            seen_gross |= state.gross_slip;
        }
        assert!(seen_gross);
        assert!((state.tangential_force_n + friction).abs() < 1.0e-12 * friction);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT / 100.0);
    }

    /// Step size: the ceiling and a sixteenth of it reach the same lift to
    /// second order, and the floor's ledger closes.
    #[test]
    fn the_ceiling_converges_and_the_floor_closes_its_ledgers() {
        // The same times: 350 and 233 steps of the ceiling, and sixteen
        // times as many of a sixteenth of it.
        let segments = |k: usize| [(350 * k, load(4.0, 2.0)), (233 * k, load(4.0, 0.0))];
        let (coarse, _) = run(&segments(1), gate());
        let (fine, _) = run(&segments(16), gate() / 16.0);
        let a = coarse.last().unwrap();
        let b = fine.last().unwrap();
        assert!((a.t_s - b.t_s).abs() < 1.0e-12);
        assert!((a.object_height_m - b.object_height_m).abs() < 2.0e-5, "{} {}", a.object_height_m, b.object_height_m);
        assert!((a.object_velocity_m_s - b.object_velocity_m_s).abs() < 2.0e-3, "{} {}", a.object_velocity_m_s, b.object_velocity_m_s);
        let floor = card().floor_dt_s().unwrap();
        let (states, stop) = run(&[(20_000, load(4.0, 2.0))], floor);
        assert_eq!(stop, None);
        assert!(states.iter().all(|s| s.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT / 100.0));
    }

    /// The ledgers' scales are what is in play: a ten-minute hold in the air
    /// at the worked grip, 700,000 steps of the ceiling, closes both ledgers
    /// at every step, where scales fixed at 2 m g H and m sqrt(4 g H)
    /// refused it at 92.1 s. At its end an unbooked move of 1e-8 of what is
    /// in play is still named by each ledger.
    #[test]
    fn a_ten_minute_hold_closes_both_ledgers_and_still_names_an_unbooked_move() {
        let card = card();
        let dt = gate();
        let mut state = GraspLiftGlass4tuState::try_start(&card, 0.0).unwrap();
        let mut frame = blank();
        let mut worst = 0.0_f64;
        for (steps, acceleration) in [(350, 2.0), (350, -2.0), (700_000, 0.0)] {
            for _ in 0..steps {
                step(&card, &mut state, &mut frame, load(1.89, acceleration), dt).unwrap();
                worst = worst.max(state.residual_conservation_error);
            }
        }
        assert!(state.t_s > 600.0 && !state.on_table && !state.slipped, "{state:?}");
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT / 10.0, "{worst:e}");
        // What is in play: about 1,200 N s of each of the pads' and gravity's impulse.
        assert!(state.momentum_scale_n_s > 2000.0, "{}", state.momentum_scale_n_s);
        let held = load(1.89, 0.0);
        let mut moved = state;
        moved.contact_impulse_n_s += 1.0e-8 * state.momentum_scale_n_s;
        assert_eq!(step(&card, &mut moved, &mut frame, held, dt), Err(StepReject::MomentumLedgerBroken));
        let mut moved = state;
        moved.object_velocity_m_s += 1.0e-8 * state.momentum_scale_n_s / OBJECT_MASS_KG;
        let got = step(&card, &mut moved, &mut frame, held, dt);
        assert!(
            matches!(got, Err(StepReject::MomentumLedgerBroken | StepReject::SpontaneousGeneration | StepReject::FirstLawBreach)),
            "{got:?}"
        );
        let mut moved = state;
        moved.energy_anchor_j -= 1.0e-8 * state.energy_scale_j;
        assert_eq!(step(&card, &mut moved, &mut frame, held, dt), Err(StepReject::SpontaneousGeneration));
        let mut moved = state;
        moved.friction_heat_j -= 1.0e-8 * state.energy_scale_j;
        assert_eq!(step(&card, &mut moved, &mut frame, held, dt), Err(StepReject::FirstLawBreach));
    }

    /// A long lift: up to 10 m/s and held there for 600,000 steps, 515 s and
    /// 5.2 km, closes both ledgers at every step, where the fixed scale
    /// refused it at 52.7 s with a defect of 1.2e-10 J against 1,025 J of
    /// the hand's work. Then the same height lowered at 10 m/s: the hand
    /// takes the work back, the work it gave and took both stay in play, and
    /// the run ends where the object comes down on the table, its ledgers
    /// closed.
    #[test]
    fn a_long_lift_and_its_lowering_close_both_ledgers() {
        let card = card();
        let dt = gate();
        let mut state = GraspLiftGlass4tuState::try_start(&card, 0.0).unwrap();
        let mut frame = blank();
        let mut worst = 0.0_f64;
        for (steps, acceleration) in [(2331, 5.0), (600_000, 0.0)] {
            for _ in 0..steps {
                step(&card, &mut state, &mut frame, load(5.0, acceleration), dt).unwrap();
                worst = worst.max(state.residual_conservation_error);
            }
        }
        assert!(state.object_height_m > 5000.0 && state.hand_work_j > 10_000.0, "{state:?}");
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT / 10.0, "{worst:e}");
        assert_eq!(state.hand_work_out_j, 0.0);
        // Turn round: down to -10 m/s and lowered until the object lands.
        let mut stop = None;
        'down: for (steps, acceleration) in [(4662, -5.0), (usize::MAX, 0.0)] {
            for _ in 0..steps {
                let before = state;
                match step(&card, &mut state, &mut frame, load(5.0, acceleration), dt) {
                    Ok(()) => worst = worst.max(state.residual_conservation_error),
                    Err(reject) => {
                        assert_eq!(state, before);
                        stop = Some(reject);
                        break 'down;
                    }
                }
            }
        }
        assert_eq!(stop, Some(StepReject::ObjectLanded));
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT / 10.0, "{worst:e}");
        // The hand took back nearly all it gave; the net is small, what was
        // in play is not.
        assert!(state.hand_work_out_j > 0.99 * state.hand_work_in_j, "{state:?}");
        assert!(state.energy_scale_j > 2.0 * state.hand_work_in_j * 0.99);
    }

    /// The hand moved down along the object resting on the table: the pads
    /// slide down its faces to their lower end, which is at the table's
    /// surface. The fingers reach the table: refused by name, the state
    /// whole, and not an end, since the object was never let go. Lifted,
    /// the object whose pads leave the lower end of its faces above the
    /// table is let go: that is the end.
    #[test]
    fn the_pads_reaching_the_table_are_refused_and_pads_leaving_a_lifted_object_end() {
        let card = card();
        let dt = gate();
        let mut state = GraspLiftGlass4tuState::try_start(&card, -0.1).unwrap();
        let mut frame = blank();
        let mut stop = None;
        for _ in 0..2000 {
            let before = state;
            let before_frame = frame;
            match step(&card, &mut state, &mut frame, load(1.89, 0.0), dt) {
                Ok(()) => assert!(state.on_table && frame.flags & FLAG_ON_TABLE != 0),
                Err(reject) => {
                    assert_eq!(state, before);
                    assert_eq!(frame.to_bytes(), before_frame.to_bytes());
                    stop = Some(reject);
                    break;
                }
            }
        }
        assert_eq!(stop, Some(StepReject::PadsReachTable));
        assert!(state.pad_travel_m > -FACE_HALF_HEIGHT_M && state.pad_travel_m < -0.029, "{}", state.pad_travel_m);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        // Lifted 0.1 m and then the hand thrown down under a weak grip.
        let (states, _) = run(&[(400, load(4.0, 1.0)), (100, load(4.0, 0.0))], dt);
        let mut lifted = *states.last().unwrap();
        assert!(lifted.object_height_m > 0.05);
        let mut stop = None;
        for _ in 0..2000 {
            let before = lifted;
            if let Err(reject) = step(&card, &mut lifted, &mut frame, load(0.5, -50.0), dt) {
                assert_eq!(lifted, before);
                stop = Some(reject);
                break;
            }
        }
        assert_eq!(stop, Some(StepReject::ObjectSlippedOut));
        assert!(lifted.object_height_m > 0.0 && lifted.pad_travel_m < -0.029, "{lifted:?}");
    }

    /// The work the hand gives and takes back are each booked: either moved
    /// unbooked is named by the energy ledger, and the net work published is
    /// their difference.
    #[test]
    fn the_two_sums_of_the_hands_work_are_each_held_by_the_ledger() {
        let card = card();
        let (states, _) = lift(4.0, gate());
        let start = states[400];
        assert_eq!(start.hand_work_j, start.hand_work_in_j - start.hand_work_out_j);
        let mut frame = blank();
        let mut moved = start;
        moved.hand_work_in_j += 1.0e-6;
        assert_eq!(step(&card, &mut moved, &mut frame, load(4.0, 0.0), gate()), Err(StepReject::FirstLawBreach));
        let mut moved = start;
        moved.hand_work_out_j += 1.0e-6;
        assert_eq!(step(&card, &mut moved, &mut frame, load(4.0, 0.0), gate()), Err(StepReject::SpontaneousGeneration));
    }

    #[test]
    fn the_same_run_gives_the_same_bits() {
        let once = || {
            let (states, _) = lift(2.5, gate());
            let mut frame = blank();
            publish(states.last().unwrap(), &mut frame);
            (frame.to_bytes(), states.len())
        };
        assert_eq!(once(), once());
    }
}
