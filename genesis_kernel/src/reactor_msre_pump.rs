#![no_implicit_prelude]

//! The Molten-Salt Reactor Experiment (MSRE, Oak Ridge National Laboratory)
//! at zero power with its 1965-66 U-235 fuel salt, held critical by the flux
//! servo while the fuel pump starts or coasts down: the delayed-neutron
//! precursors are carried around the primary loop with the salt, and those
//! that decay outside the core give no neutrons there. The card answers the
//! reactivity the control rods must supply above their static critical
//! position.
//!
//! Sources. B. E. Prince, J. R. Engel, S. J. Ball, P. N. Haubenreich and
//! T. W. Kerlin, "Zero-Power Physics Experiments on the Molten-Salt Reactor
//! Experiment", ORNL-4233 (February 1968): Table 2 (delayed-neutron decay
//! constants and effective static fractions), Fig. 2 (the 79 in effective
//! core and the vessel elevations), the 10 W servo level. B. E. Prince,
//! ORNL-TM-1626 (October 1966), Table 1: fuel residence times at full flow,
//! lower plenum 3.8 s, graphite-moderated region 9.4 s, upper plenum 3.9 s,
//! external system 8.1 s, 25.2 s around the loop.
//!
//! Calibrated on the ORNL-4233 record (evidence/research/reactor_msre_pump/
//! calibration_2026-10-18): two constants. The axial flux and importance
//! shape is sin(pi (z - z0) / H), centred where the 79 in effective core is
//! centred (39.5 in) and H = 91.6 in high, so that it reaches 6.3 in into
//! each plenum: fitted to the measured steady circulation loss, 0.212 % dk/k
//! (ORNL-4233 section 3.6). The fuel flow follows the pump speed with a
//! first-order lag of 2.55 s, fitted to the 48 rod readings of the start-up
//! (Fig. 24); the lag also takes up the speed of the rod drive, which the card
//! does not carry. The 73 rod readings of the coast-down (Fig. 24) were left
//! out of the fit. With the constants read from the sources alone (H = 79 in
//! from z = 0, no lag) the card gave a steady loss of 0.2546 % dk/k and rod
//! curves 72.4 and 36.7 pcm root mean square from the readings; calibrated,
//! 0.2120 % dk/k, 24.8 pcm on the start-up and 8.1 pcm on the coast-down.
//!
//! ```text
//! path        tau in [0, 25.2) s of residence at full flow from the lower-plenum inlet
//! height      z(tau) linear in each region: -7.15 to 5.1 in (lower plenum), 5.1 to 70.6
//!             (graphite), 70.6 to 92.475 (upper plenum, to the outlet); outside: none
//! shape       phi(z) = sin(pi (z + 6.3) / 91.6) for -6.3 <= z <= 85.3 in, else 0 (flux = importance)
//! source      s(tau) = phi / I1,   w(tau) = phi I1 / I2,   I1 = ∫ phi dtau, I2 = ∫ phi² dtau
//! parcels     252 parcels of 0.1 s move with the salt: tau_k = (k + 1/2) 0.1 + Theta (mod 25.2)
//! fuel flow   df/dt = (p − f) / 2.55 s, p the pump speed, linear within a step (exact)
//! transport   dTheta/dt = f, f taken linear within a step between its two ends
//! precursors  dC_ik/dt = beta_i P s(tau_k) 0.1 − lambda_i C_ik        (C in W s, times Lambda)
//! answer      rho = Σ_i beta_i − Σ_i Σ_k lambda_i C_ik w(tau_k) / P   (dk/k, given as percent)
//! ```
//!
//! `P` is held by the servo at 10 W (zero power: no temperature or power
//! feedback; the answer does not depend on `P`). The start is the
//! equilibrium at a steady pump speed: static at 0, circulating at any speed
//! above it, in closed form. `rho` less its value at the start is the rods'
//! compensation. Plug flow in every region; one axial shape for flux and
//! importance; no radial variation, no helium bubbles, no servo dynamics, no
//! temperature.
//!
//! Range: the pump speed from stopped (0) to full speed (1), past which the
//! card stops by name. The record compared with: starts at 0 and 1, the pump
//! speed rising no faster than 1.3 of full speed a second and falling no
//! faster than 0.81 a second (the fastest of Figs. 22 and 23, rounded up), 70 s
//! from the start (the time axis of Fig. 24); past any of these a step lands
//! and carries `FLAG_OUTSIDE_COMPARED_RUNS`.
//!
//! Ledger, per group, every step: born = decayed in the vessel + decayed
//! outside it + change in inventory. Born is integrated over the step by
//! Simpson's rule on the parcels' total source at the step's start, middle and
//! end; decayed by the two-point Hermite rule from the inventory and its rate
//! at the step's two ends. The ledger never sees how the Runge–Kutta step
//! combined its stages, so a wrong method or a falsified term shows in the
//! balance. The vessel and outside parts take each parcel's side at the
//! step's midpoint. The residual is the largest group balance relative to
//! that group's inventory. It shows the arithmetic is right; it cannot show
//! the physics matches the reactor.
//!
//! [`ReactorMsrePump::gate_dt_s`] is 0.05 s, half a parcel's passage at full
//! flow; [`ReactorMsrePump::floor_dt_s`] is 1e-6 s. The public step applies
//! neither. `core` only, with `sin`, `cos`, `exp` and `exp_m1` from
//! [`crate::math`].

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each, normal_or_zero_rows};

/// Effective static delayed-neutron fractions `beta_i` (absolute), ORNL-4233 Table 2.
pub const GROUP_FRACTION: [f64; 6] = [2.23e-4, 14.57e-4, 13.07e-4, 26.28e-4, 7.66e-4, 2.80e-4];
/// Decay constants `lambda_i`, per second, ORNL-4233 Table 2.
pub const GROUP_DECAY_PER_S: [f64; 6] = [0.0124, 0.0305, 0.1114, 0.3013, 1.140, 3.010];
/// Number of parcels the loop is cut into.
pub const PARCELS: usize = 252;

/// Power held by the servo, W: the servo level of the zero-power tests. The answer does not depend on it.
pub const POWER_W: f64 = 10.0;
/// Residence times at full flow, s (ORNL-TM-1626 Table 1).
pub const LOWER_PLENUM_S: f64 = 3.8;
pub const GRAPHITE_S: f64 = 9.4;
pub const UPPER_PLENUM_S: f64 = 3.9;
pub const EXTERNAL_S: f64 = 8.1;
/// Around the loop, s (ORNL-TM-1626 Table 1, total).
pub const LOOP_S: f64 = 25.2;
/// Residence time of one parcel at full flow, s: the loop over 252.
pub const PARCEL_S: f64 = 0.1;
/// Heights above the bottom of the effective core (827 ft 7.15 in), inches, ORNL-4233 Fig. 2:
/// the bottom of the lower head (827 ft 0 in), the bottom and the top of the graphite
/// (828 ft 0.25 in, 833 ft 5.75 in), the reactor outlet (835 ft 3.625 in).
pub const HEAD_BOTTOM_IN: f64 = -7.15;
pub const GRAPHITE_BOTTOM_IN: f64 = 5.1;
pub const GRAPHITE_TOP_IN: f64 = 70.6;
pub const OUTLET_IN: f64 = 92.475;
/// Bottom of the axial flux and importance shape, inches: calibrated, centred with the 79 in
/// effective core of Fig. 2 (39.5 in) and 91.6 in high.
pub const FLUX_BOTTOM_IN: f64 = -6.3;
/// Height of the axial flux and importance shape, inches: calibrated so that the steady
/// circulation loss at full flow is the measured 0.212 % dk/k (ORNL-4233 section 3.6).
pub const FLUX_HEIGHT_IN: f64 = 91.6;
/// First-order lag of the fuel flow behind the fuel-pump speed, s: calibrated on the 48 rod
/// readings of the start-up of Fig. 24.
pub const FUEL_FLOW_LAG_S: f64 = 2.55;
/// Fastest rise of the pump speed compared, per second: the record's 1.2231 (Fig. 22) rounded up.
pub const PUMP_RISE_COMPARED_PER_S: f64 = 1.3;
/// Fastest fall of the pump speed compared, per second: the record's 0.8079 (Fig. 23) rounded up.
pub const PUMP_FALL_COMPARED_PER_S: f64 = 0.81;
/// Rounding allowance on a change of pump speed, fraction of full speed.
pub const PUMP_CHANGE_ALLOWANCE: f64 = 1.0e-12;
/// Last time compared, s (the time axis of Fig. 24), and its rounding allowance.
pub const TIME_COMPARED_S: f64 = 70.0;
pub const TIME_ALLOWANCE_S: f64 = 1.0e-9;
/// The ledger's residual must stay at or under this: about ten times the largest the card's own
/// step leaves at the ceiling at the corners of the compared range. Euler's method and
/// Runge–Kutta with equal weights leave more on a start-up. It names a wrong method; it is not a
/// bound on the error of the state.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.0e-5;
/// Largest step the card stands behind, s.
pub const GATE_DT_S: f64 = 0.05;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-6;

/// The fuel flow changed during this step.
pub const FLAG_FLOW_CHANGING: u64 = 1 << 0;
/// The salt is moving at the end of this step.
pub const FLAG_FUEL_CIRCULATING: u64 = 1 << 1;
/// The step lies outside the runs compared with the record: a start other than the pump stopped or at full speed, the pump speed rising faster than 1.3 or falling faster than 0.81 of full speed a second, or a landing later than 70 s after the start.
pub const FLAG_OUTSIDE_COMPARED_RUNS: u64 = 1 << 2;

/// Why a step did not advance the reactor or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` or the pump speed was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this reactor's.
    InvalidSpecimen,
    /// The pump speed or the fuel flow is below zero or above the pump's full speed, incoming, in the load or landing.
    FlowOutsideCard,
    /// The incoming state holds a precursor content or a ledger term below zero.
    NegativeState,
    /// The incoming state is not one this card writes: at time zero it is not the card's start; later, an anchor or a derived field is not what the card computes from the rest of the state.
    InconsistentState,
    /// The trial step created precursors beyond what the ledger supplies.
    SpontaneousGeneration,
    /// The trial step destroyed precursors the ledger supplies.
    FirstLawBreach,
}

/// What the pump does over one step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorMsrePumpLoad {
    /// Fuel-pump speed as a fraction of full speed at the end of the step; the speed is linear within the step.
    pub pump_speed_fraction: f64,
}

/// The locked reactor. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorMsrePump {
    pub group_fraction: [f64; 6],
    pub group_decay_per_s: [f64; 6],
    pub constants: [f64; 25],
}

const SCALARS: [f64; 25] = [
    POWER_W,
    LOWER_PLENUM_S,
    GRAPHITE_S,
    UPPER_PLENUM_S,
    EXTERNAL_S,
    LOOP_S,
    PARCEL_S,
    HEAD_BOTTOM_IN,
    GRAPHITE_BOTTOM_IN,
    GRAPHITE_TOP_IN,
    OUTLET_IN,
    FLUX_HEIGHT_IN,
    FLUX_BOTTOM_IN,
    FUEL_FLOW_LAG_S,
    PUMP_RISE_COMPARED_PER_S,
    PUMP_FALL_COMPARED_PER_S,
    PUMP_CHANGE_ALLOWANCE,
    TIME_COMPARED_S,
    TIME_ALLOWANCE_S,
    RESIDUAL_CONSERVATION_LIMIT,
    GATE_DT_S,
    FLOOR_DT_S,
    PARCELS as f64,
    GROUP_FRACTION[0] + GROUP_FRACTION[1] + GROUP_FRACTION[2] + GROUP_FRACTION[3] + GROUP_FRACTION[4] + GROUP_FRACTION[5],
    1.0,
];

const K_POWER: usize = 0;
const K_LOWER: usize = 1;
const K_GRAPHITE: usize = 2;
const K_UPPER: usize = 3;
const K_LOOP: usize = 5;
const K_PARCEL: usize = 6;
const K_HEAD: usize = 7;
const K_GBOT: usize = 8;
const K_GTOP: usize = 9;
const K_OUTLET: usize = 10;
const K_HEIGHT: usize = 11;
const K_BOTTOM: usize = 12;
const K_LAG: usize = 13;
const K_RISE: usize = 14;
const K_FALL: usize = 15;
const K_CHANGE_ALLOW: usize = 16;
const K_TIME: usize = 17;
const K_TIME_ALLOW: usize = 18;
const K_RESIDUAL: usize = 19;
const K_GATE: usize = 20;
const K_FLOOR: usize = 21;
const K_FULL_SPEED: usize = 24;

const PI: f64 = 3.141592653589793;

/// A stretch of the path on which phi = sin(a tau + b).
#[derive(Clone, Copy)]
struct Piece {
    from: f64,
    to: f64,
    a: f64,
    b: f64,
}

/// The flux shape on the path, normalised.
#[derive(Clone, Copy)]
struct Shape {
    pieces: [Piece; 3],
    /// 1 / ∫ phi dtau.
    source_scale: f64,
    /// ∫ phi dtau / ∫ phi² dtau.
    weight_scale: f64,
    /// End of the vessel on the path: lower plenum + graphite + upper plenum.
    vessel_end: f64,
    loop_s: f64,
}

impl ReactorMsrePump {
    /// The MSRE at zero power with the fuel pump starting or coasting down. No arguments.
    #[must_use]
    pub fn specimen() -> Self {
        let reactor = Self { group_fraction: GROUP_FRACTION, group_decay_per_s: GROUP_DECAY_PER_S, constants: SCALARS };
        if reactor.is_valid() {
            reactor
        } else {
            ::core::panic!("the MSRE reactor failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        let mut index = 0;
        while index < 6 {
            if self.group_fraction[index].to_bits() != GROUP_FRACTION[index].to_bits()
                || self.group_decay_per_s[index].to_bits() != GROUP_DECAY_PER_S[index].to_bits()
            {
                return false;
            }
            index += 1;
        }
        index = 0;
        while index < SCALARS.len() {
            if self.constants[index].to_bits() != SCALARS[index].to_bits() {
                return false;
            }
            index += 1;
        }
        true
    }

    /// Largest step the card stands behind: 0.05 s. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let gate = self.constants[K_GATE];
        if gate.is_finite() && gate > 0.0 { Some(gate) } else { None }
    }

    /// Smallest step the card stands behind: 1e-6 s. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        let floor = self.constants[K_FLOOR];
        if floor.is_finite() && floor > 0.0 { Some(floor) } else { None }
    }

    /// Whether a pump speed or a fuel flow is on the card: from stopped, 0, to full speed, 1.
    #[must_use]
    pub fn flow_on_card(&self, fraction: f64) -> bool {
        fraction >= 0.0 && fraction <= self.constants[K_FULL_SPEED]
    }

    /// Whether a change of pump speed over a step is within the compared record, rising or falling.
    #[must_use]
    pub fn pump_change_compared(&self, from: f64, to: f64, dt_s: f64) -> bool {
        to - from <= self.constants[K_RISE] * dt_s + self.constants[K_CHANGE_ALLOW]
            && from - to <= self.constants[K_FALL] * dt_s + self.constants[K_CHANGE_ALLOW]
    }

    /// The steady circulation loss at full flow, percent dk/k: the answer at the full-speed start.
    #[must_use]
    pub fn steady_loss_percent_dk_k(&self) -> f64 {
        match ReactorMsrePumpState::try_start(self, 1.0) {
            Some(state) => state.circulation_loss_percent_dk_k,
            None => f64::NAN,
        }
    }

    fn shape(&self) -> Shape {
        let c = &self.constants;
        let t1 = c[K_LOWER];
        let t2 = t1 + c[K_GRAPHITE];
        let t3 = t2 + c[K_UPPER];
        let h = c[K_HEIGHT];
        let z0 = c[K_BOTTOM];
        let regions = [
            (0.0, t1, c[K_HEAD] - z0, c[K_GBOT] - z0),
            (t1, t2, c[K_GBOT] - z0, c[K_GTOP] - z0),
            (t2, t3, c[K_GTOP] - z0, c[K_OUTLET] - z0),
        ];
        let mut pieces = [Piece { from: 0.0, to: 0.0, a: 0.0, b: 0.0 }; 3];
        let mut first = 0.0;
        let mut second = 0.0;
        let mut index = 0;
        while index < 3 {
            let (ta, tb, za, zb) = regions[index];
            let v = (zb - za) / (tb - ta);
            // the stretch of this region inside 0 <= z - z0 <= h
            let mut from = ta;
            let mut to = tb;
            if za < 0.0 {
                from = ta + (0.0 - za) / v;
            }
            if zb > h {
                to = ta + (h - za) / v;
            }
            let a = PI * v / h;
            let b = PI * (za - v * ta) / h;
            pieces[index] = Piece { from, to, a, b };
            first += (crate::math::cos(a * from + b) - crate::math::cos(a * to + b)) / a;
            second += 0.5 * (to - from)
                - (crate::math::sin(2.0 * (a * to + b)) - crate::math::sin(2.0 * (a * from + b))) / (4.0 * a);
            index += 1;
        }
        Shape { pieces, source_scale: 1.0 / first, weight_scale: first / second, vessel_end: t3, loop_s: c[K_LOOP] }
    }
}

impl Shape {
    /// phi at a path position in [0, loop).
    fn phi(&self, tau: f64) -> f64 {
        let mut index = 0;
        while index < 3 {
            let p = self.pieces[index];
            if tau >= p.from && tau <= p.to {
                return crate::math::sin(p.a * tau + p.b);
            }
            index += 1;
        }
        0.0
    }
}

/// Position of parcel `k` on the path at phase `phase`, in [0, loop).
fn position(parcel_s: f64, loop_s: f64, k: usize, phase: f64) -> f64 {
    let mut x = (k as f64 + 0.5) * parcel_s + phase;
    while x >= loop_s {
        x -= loop_s;
    }
    x
}

/// Integrated reactor. The ledger, the anchors and the derived fields are part of the state;
/// write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorMsrePumpState {
    /// Time since the start, s.
    pub t_s: f64,
    /// Pump speed of the start's equilibrium, fraction of full speed.
    pub pump_speed_at_start: f64,
    /// Pump speed at this state, fraction of full speed.
    pub pump_speed_fraction: f64,
    /// Fuel flow at this state, fraction of full flow.
    pub fuel_flow_fraction: f64,
    /// How far the salt has moved, in seconds of full-flow travel, modulo the loop.
    pub phase_s: f64,
    /// Precursor content of each parcel, W s (times the generation time).
    pub precursor_ws: [[f64; PARCELS]; 6],
    /// Each group's inventory at the start.
    pub inventory_anchor_ws: [f64; 6],
    /// Precursors born since the start, per group.
    pub born_ws: [f64; 6],
    /// Precursors decayed in the reactor vessel since the start, per group.
    pub decayed_vessel_ws: [f64; 6],
    /// Precursors decayed outside the vessel since the start, per group.
    pub decayed_outside_ws: [f64; 6],
    /// Signed balance per group: inventory − anchor − born + decayed; positive is created.
    pub group_defect_ws: [f64; 6],
    /// Largest group balance relative to that group's inventory.
    pub residual_conservation_error: f64,
    /// Reactivity the rods supply above the static critical position, percent dk/k.
    pub circulation_loss_percent_dk_k: f64,
    /// That at the start.
    pub loss_at_start_percent_dk_k: f64,
    /// The rods' compensation since the start: the loss less the loss at the start, percent dk/k.
    pub rod_compensation_percent_dk_k: f64,
    /// Mean rate of change of the fuel flow over the last step, per s. Zero at the start.
    pub fuel_flow_rate_per_s: f64,
    /// The fuel flow changed during the last step.
    pub flow_changing: bool,
    /// The salt is moving.
    pub fuel_circulating: bool,
    /// The last step lay outside the runs compared with the record.
    pub outside_compared_runs: bool,
}

impl ReactorMsrePumpState {
    /// The reactor in equilibrium at a steady pump speed `pump_speed_at_start` (0, static; above
    /// 0, circulating), time 0. `None` unless the speed is finite and from 0 to full speed.
    #[must_use]
    pub fn try_start(reactor: &ReactorMsrePump, pump_speed_at_start: f64) -> Option<Self> {
        if !reactor.is_valid() || !pump_speed_at_start.is_finite() || !reactor.flow_on_card(pump_speed_at_start) {
            return None;
        }
        let shape = reactor.shape();
        let parcel_s = reactor.constants[K_PARCEL];
        let power_w = reactor.constants[K_POWER];
        let f = pump_speed_at_start;
        let mut precursor_ws = [[0.0; PARCELS]; 6];
        let mut group = 0;
        while group < 6 {
            let beta = reactor.group_fraction[group];
            let lambda = reactor.group_decay_per_s[group];
            let mut k = 0;
            while k < PARCELS {
                let tau = position(parcel_s, shape.loop_s, k, 0.0);
                precursor_ws[group][k] = if f == 0.0 {
                    beta * power_w * parcel_s * shape.phi(tau) * shape.source_scale / lambda
                } else {
                    beta * power_w * parcel_s * shape.source_scale * circulating(&shape, lambda, f, tau)
                };
                k += 1;
            }
            group += 1;
        }
        normal_or_zero_rows(&mut precursor_ws);
        let mut state = Self {
            t_s: 0.0,
            pump_speed_at_start: f,
            pump_speed_fraction: f,
            fuel_flow_fraction: f,
            phase_s: 0.0,
            precursor_ws,
            inventory_anchor_ws: [0.0; 6],
            born_ws: [0.0; 6],
            decayed_vessel_ws: [0.0; 6],
            decayed_outside_ws: [0.0; 6],
            group_defect_ws: [0.0; 6],
            residual_conservation_error: 0.0,
            circulation_loss_percent_dk_k: 0.0,
            loss_at_start_percent_dk_k: 0.0,
            rod_compensation_percent_dk_k: 0.0,
            fuel_flow_rate_per_s: 0.0,
            flow_changing: false,
            fuel_circulating: f > 0.0,
            outside_compared_runs: false,
        };
        let mut group = 0;
        while group < 6 {
            state.inventory_anchor_ws[group] = normal_or_zero(inventory(&state.precursor_ws[group]));
            group += 1;
        }
        let loss = normal_or_zero(loss_percent(reactor, &shape, &state));
        state.circulation_loss_percent_dk_k = loss;
        state.loss_at_start_percent_dk_k = loss;
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }
}

/// The steady content per unit source of the salt now at `tau` at a steady fuel flow `f` above
/// zero: ∫ over one period of the path of phi(u) e^(−lambda d(u) / f) du / f, over
/// (1 − e^(−lambda T / f)), with d(u) the path from `u` to `tau`.
fn circulating(shape: &Shape, lambda: f64, f: f64, tau: f64) -> f64 {
    let t = shape.loop_s;
    let mut sum = 0.0;
    let mut index = 0;
    while index < 3 {
        let p = shape.pieces[index];
        // the part of the piece passed before reaching tau this lap, and the part passed last lap
        if p.from < tau {
            let upper = if p.to < tau { p.to } else { tau };
            sum += decayed_sine(p, lambda, f, p.from, upper, tau);
        }
        if p.to > tau {
            let lower = if p.from > tau { p.from } else { tau };
            sum += decayed_sine(p, lambda, f, lower, p.to, tau + t);
        }
        index += 1;
    }
    sum / -crate::math::exp_m1(-lambda * t / f)
}

/// (1 / f) ∫_from^to sin(a u + b) e^(−lambda (at − u) / f) du, in closed form, written so that
/// no term divides by `f` but the exponent's.
fn decayed_sine(p: Piece, lambda: f64, f: f64, from: f64, to: f64, at: f64) -> f64 {
    let denominator = lambda * lambda + p.a * p.a * f * f;
    let end = |u: f64| {
        let arg = p.a * u + p.b;
        crate::math::exp(-lambda * (at - u) / f) * (lambda * crate::math::sin(arg) - p.a * f * crate::math::cos(arg)) / denominator
    };
    end(to) - end(from)
}

fn inventory(parcels: &[f64; PARCELS]) -> f64 {
    let mut sum = 0.0;
    let mut k = 0;
    while k < PARCELS {
        sum += parcels[k];
        k += 1;
    }
    sum
}

/// rho in percent dk/k at a state.
fn loss_percent(reactor: &ReactorMsrePump, shape: &Shape, state: &ReactorMsrePumpState) -> f64 {
    let parcel_s = reactor.constants[K_PARCEL];
    let power_w = reactor.constants[K_POWER];
    let mut weights = [0.0; PARCELS];
    let mut k = 0;
    while k < PARCELS {
        weights[k] = shape.phi(position(parcel_s, shape.loop_s, k, state.phase_s)) * shape.weight_scale;
        k += 1;
    }
    let mut rho = 0.0;
    let mut group = 0;
    while group < 6 {
        let mut emitted = 0.0;
        k = 0;
        while k < PARCELS {
            emitted += state.precursor_ws[group][k] * weights[k];
            k += 1;
        }
        rho += reactor.group_fraction[group] - reactor.group_decay_per_s[group] * emitted / power_w;
        group += 1;
    }
    100.0 * rho
}

type Field = [[f64; PARCELS]; 6];

/// The parcels' sources s(tau_k) 0.1 s at a phase.
fn sources(reactor: &ReactorMsrePump, shape: &Shape, phase: f64) -> [f64; PARCELS] {
    let parcel_s = reactor.constants[K_PARCEL];
    let mut s = [0.0; PARCELS];
    let mut k = 0;
    while k < PARCELS {
        s[k] = shape.phi(position(parcel_s, shape.loop_s, k, phase)) * shape.source_scale * parcel_s;
        k += 1;
    }
    s
}

fn rates(reactor: &ReactorMsrePump, c: &Field, s: &[f64; PARCELS]) -> Field {
    let mut out = [[0.0; PARCELS]; 6];
    let power_w = reactor.constants[K_POWER];
    let mut group = 0;
    while group < 6 {
        let born = reactor.group_fraction[group] * power_w;
        let lambda = reactor.group_decay_per_s[group];
        let mut k = 0;
        while k < PARCELS {
            out[group][k] = born * s[k] - lambda * c[group][k];
            k += 1;
        }
        group += 1;
    }
    out
}

fn advance(c: &Field, k: &Field, scale: f64) -> Field {
    let mut out = *c;
    let mut group = 0;
    while group < 6 {
        let mut p = 0;
        while p < PARCELS {
            out[group][p] = c[group][p] + scale * k[group][p];
            p += 1;
        }
        group += 1;
    }
    out
}

/// The phase reached a time `into` after the step's start, with the fuel flow linear over the step.
fn phase_at(phase: f64, from: f64, to: f64, dt_s: f64, into: f64) -> f64 {
    phase + from * into + 0.5 * (to - from) * into * into / dt_s
}

/// The fuel flow at the end of a step: df/dt = (p − f) / lag with the pump speed p linear from
/// `p0` to `p1` over the step, exactly: f0 e + p0 (1 − e) + s (dt − lag (1 − e)), e = e^(−dt/lag).
fn fuel_flow_after(reactor: &ReactorMsrePump, f0: f64, p0: f64, p1: f64, dt_s: f64) -> f64 {
    let lag = reactor.constants[K_LAG];
    let y = dt_s / lag;
    let one_minus_e = -crate::math::exp_m1(-y);
    let slope = (p1 - p0) / dt_s;
    f0 + (p0 - f0) * one_minus_e + slope * lag * (y - one_minus_e)
}

/// The sources at the step's start, middle and end.
struct StepSources {
    start: [f64; PARCELS],
    middle: [f64; PARCELS],
    end: [f64; PARCELS],
}

/// Classical fourth-order Runge–Kutta with the source taken at the step's start, middle and end.
fn rk4(reactor: &ReactorMsrePump, c: &Field, sources: &StepSources, dt_s: f64) -> Field {
    let half = 0.5 * dt_s;
    let k1 = rates(reactor, c, &sources.start);
    let k2 = rates(reactor, &advance(c, &k1, half), &sources.middle);
    let k3 = rates(reactor, &advance(c, &k2, half), &sources.middle);
    let k4 = rates(reactor, &advance(c, &k3, dt_s), &sources.end);
    let mut out = *c;
    let mut group = 0;
    while group < 6 {
        let mut p = 0;
        while p < PARCELS {
            out[group][p] = c[group][p] + dt_s / 6.0 * (k1[group][p] + 2.0 * k2[group][p] + 2.0 * k3[group][p] + k4[group][p]);
            p += 1;
        }
        group += 1;
    }
    out
}

type Method = fn(&ReactorMsrePump, &Field, &StepSources, f64) -> Field;

/// Advance one step and write the frame. A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    reactor: &ReactorMsrePump,
    state: &mut ReactorMsrePumpState,
    frame: &mut LastStateFrame64,
    load: ReactorMsrePumpLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(rk4, reactor, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

fn normal_or_zero_state(state: &mut ReactorMsrePumpState) {
    state.t_s = normal_or_zero(state.t_s);
    state.pump_speed_at_start = normal_or_zero(state.pump_speed_at_start);
    state.pump_speed_fraction = normal_or_zero(state.pump_speed_fraction);
    state.fuel_flow_fraction = normal_or_zero(state.fuel_flow_fraction);
    state.phase_s = normal_or_zero(state.phase_s);
    normal_or_zero_rows(&mut state.precursor_ws);
    normal_or_zero_each(&mut state.inventory_anchor_ws);
    normal_or_zero_each(&mut state.born_ws);
    normal_or_zero_each(&mut state.decayed_vessel_ws);
    normal_or_zero_each(&mut state.decayed_outside_ws);
    normal_or_zero_each(&mut state.group_defect_ws);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.circulation_loss_percent_dk_k = normal_or_zero(state.circulation_loss_percent_dk_k);
    state.loss_at_start_percent_dk_k = normal_or_zero(state.loss_at_start_percent_dk_k);
    state.rod_compensation_percent_dk_k = normal_or_zero(state.rod_compensation_percent_dk_k);
    state.fuel_flow_rate_per_s = normal_or_zero(state.fuel_flow_rate_per_s);
}

fn step_as_computed(
    method: Method,
    reactor: &ReactorMsrePump,
    state: &mut ReactorMsrePumpState,
    frame: &mut LastStateFrame64,
    load: ReactorMsrePumpLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(reactor, state, load, dt_s)?;
    let shape = reactor.shape();
    let f0 = state.fuel_flow_fraction;
    let f1 = fuel_flow_after(reactor, f0, state.pump_speed_fraction, load.pump_speed_fraction, dt_s);
    if !f1.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !reactor.flow_on_card(f1) {
        return Err(StepReject::FlowOutsideCard);
    }
    let srcs = StepSources {
        start: sources(reactor, &shape, state.phase_s),
        middle: sources(reactor, &shape, phase_at(state.phase_s, f0, f1, dt_s, 0.5 * dt_s)),
        end: sources(reactor, &shape, phase_at(state.phase_s, f0, f1, dt_s, dt_s)),
    };
    let advanced = method(reactor, &state.precursor_ws, &srcs, dt_s);
    commit(reactor, &shape, state, frame, load, f1, dt_s, &advanced, &srcs)
}

fn validate(reactor: &ReactorMsrePump, state: &ReactorMsrePumpState, load: ReactorMsrePumpLoad, dt_s: f64) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.pump_speed_fraction.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !reactor.is_valid() {
        return Err(StepReject::InvalidSpecimen);
    }
    if !reactor.flow_on_card(load.pump_speed_fraction)
        || !reactor.flow_on_card(state.pump_speed_fraction)
        || !reactor.flow_on_card(state.fuel_flow_fraction)
        || !reactor.flow_on_card(state.pump_speed_at_start)
    {
        return Err(StepReject::FlowOutsideCard);
    }
    incoming_state_is_the_cards(reactor, state)
}

fn incoming_state_is_the_cards(reactor: &ReactorMsrePump, state: &ReactorMsrePumpState) -> Result<(), StepReject> {
    let mut group = 0;
    while group < 6 {
        let mut k = 0;
        while k < PARCELS {
            if state.precursor_ws[group][k] < 0.0 {
                return Err(StepReject::NegativeState);
            }
            k += 1;
        }
        if state.born_ws[group] < 0.0 || state.decayed_vessel_ws[group] < 0.0 || state.decayed_outside_ws[group] < 0.0 {
            return Err(StepReject::NegativeState);
        }
        group += 1;
    }
    let start = match ReactorMsrePumpState::try_start(reactor, state.pump_speed_at_start) {
        Some(start) => start,
        None => return Err(StepReject::InconsistentState),
    };
    if state.t_s == 0.0 {
        return if same_bits(state, &start) { Ok(()) } else { Err(StepReject::InconsistentState) };
    }
    if !(state.t_s > 0.0) {
        return Err(StepReject::InconsistentState);
    }
    let shape = reactor.shape();
    let (defects, residual) = balances(state);
    let loss = normal_or_zero(loss_percent(reactor, &shape, state));
    let mut group = 0;
    while group < 6 {
        if state.inventory_anchor_ws[group].to_bits() != start.inventory_anchor_ws[group].to_bits()
            || state.group_defect_ws[group].to_bits() != normal_or_zero(defects[group]).to_bits()
        {
            return Err(StepReject::InconsistentState);
        }
        group += 1;
    }
    if state.loss_at_start_percent_dk_k.to_bits() != start.loss_at_start_percent_dk_k.to_bits()
        || state.residual_conservation_error.to_bits() != normal_or_zero(residual).to_bits()
        || state.circulation_loss_percent_dk_k.to_bits() != loss.to_bits()
        || state.rod_compensation_percent_dk_k.to_bits() != normal_or_zero(loss - start.loss_at_start_percent_dk_k).to_bits()
        || !(state.phase_s >= 0.0 && state.phase_s < reactor.constants[K_LOOP])
    {
        return Err(StepReject::InconsistentState);
    }
    Ok(())
}

/// Every field of two states equal bit for bit.
fn same_bits(a: &ReactorMsrePumpState, b: &ReactorMsrePumpState) -> bool {
    let mut group = 0;
    while group < 6 {
        let mut k = 0;
        while k < PARCELS {
            if a.precursor_ws[group][k].to_bits() != b.precursor_ws[group][k].to_bits() {
                return false;
            }
            k += 1;
        }
        if a.inventory_anchor_ws[group].to_bits() != b.inventory_anchor_ws[group].to_bits()
            || a.born_ws[group].to_bits() != b.born_ws[group].to_bits()
            || a.decayed_vessel_ws[group].to_bits() != b.decayed_vessel_ws[group].to_bits()
            || a.decayed_outside_ws[group].to_bits() != b.decayed_outside_ws[group].to_bits()
            || a.group_defect_ws[group].to_bits() != b.group_defect_ws[group].to_bits()
        {
            return false;
        }
        group += 1;
    }
    let fa = scalar_fields(a);
    let fb = scalar_fields(b);
    let mut index = 0;
    while index < fa.len() {
        if fa[index].to_bits() != fb[index].to_bits() {
            return false;
        }
        index += 1;
    }
    a.flow_changing == b.flow_changing && a.fuel_circulating == b.fuel_circulating && a.outside_compared_runs == b.outside_compared_runs
}

fn scalar_fields(s: &ReactorMsrePumpState) -> [f64; 10] {
    [
        s.t_s,
        s.pump_speed_at_start,
        s.pump_speed_fraction,
        s.fuel_flow_fraction,
        s.phase_s,
        s.residual_conservation_error,
        s.circulation_loss_percent_dk_k,
        s.loss_at_start_percent_dk_k,
        s.rod_compensation_percent_dk_k,
        s.fuel_flow_rate_per_s,
    ]
}

/// The group balances and the residual.
fn balances(state: &ReactorMsrePumpState) -> ([f64; 6], f64) {
    let mut defects = [0.0; 6];
    let mut residual = 0.0;
    let mut group = 0;
    while group < 6 {
        let now = inventory(&state.precursor_ws[group]);
        let anchor = state.inventory_anchor_ws[group];
        defects[group] =
            now - anchor - state.born_ws[group] + state.decayed_vessel_ws[group] + state.decayed_outside_ws[group];
        let scale = if now > anchor { now } else { anchor };
        let r = if scale > 0.0 { defects[group].abs() / scale } else { defects[group].abs() };
        if r > residual {
            residual = r;
        }
        group += 1;
    }
    (defects, residual)
}

/// `h (f0 + f1) / 2 + h² (f0' − f1') / 12`.
fn hermite_rule(h: f64, f0: f64, f1: f64, d0: f64, d1: f64) -> f64 {
    0.5 * h * (f0 + f1) + h * h * (d0 - d1) / 12.0
}

#[allow(clippy::too_many_arguments)]
fn commit(
    reactor: &ReactorMsrePump,
    shape: &Shape,
    state: &mut ReactorMsrePumpState,
    frame: &mut LastStateFrame64,
    load: ReactorMsrePumpLoad,
    f1: f64,
    dt_s: f64,
    advanced: &Field,
    srcs: &StepSources,
) -> Result<(), StepReject> {
    let mut group = 0;
    while group < 6 {
        let mut k = 0;
        while k < PARCELS {
            if !advanced[group][k].is_finite() {
                return Err(StepReject::NonFiniteState);
            }
            k += 1;
        }
        group += 1;
    }
    let mut next = *state;
    let f0 = state.fuel_flow_fraction;
    let parcel_s = reactor.constants[K_PARCEL];
    let power_w = reactor.constants[K_POWER];
    let phase_end_raw = phase_at(state.phase_s, f0, f1, dt_s, dt_s);
    let phase_middle = phase_at(state.phase_s, f0, f1, dt_s, 0.5 * dt_s);
    let k0 = rates(reactor, &state.precursor_ws, &srcs.start);
    let k1 = rates(reactor, advanced, &srcs.end);
    // each parcel's side of the vessel at the step's middle
    let mut in_vessel = [false; PARCELS];
    let mut k = 0;
    while k < PARCELS {
        in_vessel[k] = position(parcel_s, shape.loop_s, k, phase_middle) < shape.vessel_end;
        k += 1;
    }
    // born: Simpson's rule on the parcels' total source at the step's start, middle and end
    let mut source_total = (0.0, 0.0, 0.0);
    k = 0;
    while k < PARCELS {
        source_total.0 += srcs.start[k];
        source_total.1 += srcs.middle[k];
        source_total.2 += srcs.end[k];
        k += 1;
    }
    let born_share = dt_s / 6.0 * (source_total.0 + 4.0 * source_total.1 + source_total.2);
    group = 0;
    while group < 6 {
        let lambda = reactor.group_decay_per_s[group];
        let mut v = (0.0, 0.0, 0.0, 0.0);
        let mut o = (0.0, 0.0, 0.0, 0.0);
        k = 0;
        while k < PARCELS {
            let part = (state.precursor_ws[group][k], advanced[group][k], k0[group][k], k1[group][k]);
            if in_vessel[k] {
                v = (v.0 + part.0, v.1 + part.1, v.2 + part.2, v.3 + part.3);
            } else {
                o = (o.0 + part.0, o.1 + part.1, o.2 + part.2, o.3 + part.3);
            }
            k += 1;
        }
        next.born_ws[group] += reactor.group_fraction[group] * power_w * born_share;
        next.decayed_vessel_ws[group] += lambda * hermite_rule(dt_s, v.0, v.1, v.2, v.3);
        next.decayed_outside_ws[group] += lambda * hermite_rule(dt_s, o.0, o.1, o.2, o.3);
        group += 1;
    }
    next.precursor_ws = *advanced;
    let pump_change_compared = reactor.pump_change_compared(state.pump_speed_fraction, load.pump_speed_fraction, dt_s);
    next.t_s += dt_s;
    next.pump_speed_fraction = load.pump_speed_fraction;
    next.fuel_flow_fraction = f1;
    let mut phase = phase_end_raw;
    while phase >= shape.loop_s {
        phase -= shape.loop_s;
    }
    next.phase_s = phase;
    next.fuel_flow_rate_per_s = (f1 - f0) / dt_s;
    let (defects, residual) = balances(&next);
    next.group_defect_ws = defects;
    next.residual_conservation_error = residual;
    let loss = loss_percent(reactor, shape, &next);
    next.circulation_loss_percent_dk_k = loss;
    next.rod_compensation_percent_dk_k = loss - next.loss_at_start_percent_dk_k;
    next.flow_changing = f1 != f0;
    next.fuel_circulating = f1 > 0.0;
    let start_compared = next.pump_speed_at_start == 0.0 || next.pump_speed_at_start == reactor.constants[K_FULL_SPEED];
    next.outside_compared_runs = !start_compared
        || !pump_change_compared
        || !(next.t_s <= reactor.constants[K_TIME] + reactor.constants[K_TIME_ALLOW]);
    if !state_is_finite(&next) {
        return Err(StepReject::NonFiniteState);
    }
    if next.residual_conservation_error > reactor.constants[K_RESIDUAL] {
        return Err(if largest_defect_is_created(&next) { StepReject::SpontaneousGeneration } else { StepReject::FirstLawBreach });
    }
    *state = next;
    publish(state, frame);
    Ok(())
}

fn largest_defect_is_created(state: &ReactorMsrePumpState) -> bool {
    let mut largest = -1.0;
    let mut created = false;
    let mut group = 0;
    while group < 6 {
        let now = inventory(&state.precursor_ws[group]);
        let anchor = state.inventory_anchor_ws[group];
        let scale = if now > anchor { now } else { anchor };
        let r = if scale > 0.0 { state.group_defect_ws[group].abs() / scale } else { state.group_defect_ws[group].abs() };
        if r > largest {
            largest = r;
            created = state.group_defect_ws[group] > 0.0;
        }
        group += 1;
    }
    created
}

fn publish(state: &ReactorMsrePumpState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.rod_compensation_percent_dk_k as f32;
    frame.pos[1] = state.circulation_loss_percent_dk_k as f32;
    frame.pos[2] = state.fuel_flow_fraction as f32;
    frame.vel[0] = state.pump_speed_fraction as f32;
    frame.vel[1] = state.loss_at_start_percent_dk_k as f32;
    frame.vel[2] = state.phase_s as f32;
    frame.force_torque = state.fuel_flow_rate_per_s as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.flow_changing {
        flags |= FLAG_FLOW_CHANGING;
    }
    if state.fuel_circulating {
        flags |= FLAG_FUEL_CIRCULATING;
    }
    if state.outside_compared_runs {
        flags |= FLAG_OUTSIDE_COMPARED_RUNS;
    }
    frame.flags = flags;
}

/// Each group's inventory inside and outside the vessel at a state, W s.
#[must_use]
pub fn inventories(reactor: &ReactorMsrePump, state: &ReactorMsrePumpState) -> ([f64; 6], [f64; 6]) {
    let shape = reactor.shape();
    let parcel_s = reactor.constants[K_PARCEL];
    let mut vessel = [0.0; 6];
    let mut outside = [0.0; 6];
    let mut group = 0;
    while group < 6 {
        let mut k = 0;
        while k < PARCELS {
            if position(parcel_s, shape.loop_s, k, state.phase_s) < shape.vessel_end {
                vessel[group] += state.precursor_ws[group][k];
            } else {
                outside[group] += state.precursor_ws[group][k];
            }
            k += 1;
        }
        group += 1;
    }
    (vessel, outside)
}

fn state_is_finite(state: &ReactorMsrePumpState) -> bool {
    let mut group = 0;
    while group < 6 {
        let mut k = 0;
        while k < PARCELS {
            if !state.precursor_ws[group][k].is_finite() {
                return false;
            }
            k += 1;
        }
        if !state.inventory_anchor_ws[group].is_finite()
            || !state.born_ws[group].is_finite()
            || !state.decayed_vessel_ws[group].is_finite()
            || !state.decayed_outside_ws[group].is_finite()
            || !state.group_defect_ws[group].is_finite()
        {
            return false;
        }
        group += 1;
    }
    let fields = scalar_fields(state);
    let mut index = 0;
    while index < fields.len() {
        if !fields[index].is_finite() {
            return false;
        }
        index += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::format;

    use super::*;

    fn marked_frame() -> LastStateFrame64 {
        let mut frame = LastStateFrame64::from_bytes([0u8; 64]);
        frame.proof = [0xA5; 16];
        frame.t = -1.0;
        frame
    }

    fn card() -> ReactorMsrePump {
        ReactorMsrePump::specimen()
    }

    fn start(speed: f64) -> ReactorMsrePumpState {
        ReactorMsrePumpState::try_start(&card(), speed).unwrap()
    }

    fn load(speed: f64) -> ReactorMsrePumpLoad {
        ReactorMsrePumpLoad { pump_speed_fraction: speed }
    }

    fn euler(reactor: &ReactorMsrePump, c: &Field, s: &StepSources, dt_s: f64) -> Field {
        advance(c, &rates(reactor, c, &s.start), dt_s)
    }

    fn equal_weights(reactor: &ReactorMsrePump, c: &Field, s: &StepSources, dt_s: f64) -> Field {
        let k1 = rates(reactor, c, &s.start);
        let k2 = rates(reactor, &advance(c, &k1, 0.5 * dt_s), &s.middle);
        let k3 = rates(reactor, &advance(c, &k2, 0.5 * dt_s), &s.middle);
        let k4 = rates(reactor, &advance(c, &k3, dt_s), &s.end);
        let mut out = *c;
        for g in 0..6 {
            for p in 0..PARCELS {
                out[g][p] = c[g][p] + dt_s / 4.0 * (k1[g][p] + k2[g][p] + k3[g][p] + k4[g][p]);
            }
        }
        out
    }

    /// Runge–Kutta that leaves out a hundredth of the source (a falsified transport).
    fn leaky_source(reactor: &ReactorMsrePump, c: &Field, s: &StepSources, dt_s: f64) -> Field {
        let mut out = rk4(reactor, c, s, dt_s);
        for g in 0..6 {
            for p in 0..PARCELS {
                out[g][p] -= 0.01 * dt_s * reactor.group_fraction[g] * POWER_W * s.start[p];
            }
        }
        out
    }

    fn ramp_up(t: f64) -> f64 {
        if 1.2 * t < 1.0 { 1.2 * t } else { 1.0 }
    }

    struct Run {
        last: ReactorMsrePumpState,
        steps: u32,
        largest_residual: f64,
        end: Option<StepReject>,
    }

    fn run_with(method: Method, start_speed: f64, profile: fn(f64) -> f64, dt: f64, steps: u32) -> Run {
        let reactor = card();
        let mut state = start(start_speed);
        let mut frame = marked_frame();
        let mut landed = 0u32;
        let mut largest_residual = 0.0f64;
        let mut end = None;
        while landed < steps {
            let t = (landed + 1) as f64 * dt;
            match step_as_computed(method, &reactor, &mut state, &mut frame, load(profile(t)), dt) {
                Ok(()) => {
                    normal_or_zero_state(&mut state);
                    landed += 1;
                    largest_residual = largest_residual.max(state.residual_conservation_error);
                    assert_eq!(frame.proof, [0xA5; 16]);
                }
                Err(reject) => {
                    end = Some(reject);
                    break;
                }
            }
        }
        Run { last: state, steps: landed, largest_residual, end }
    }

    #[test]
    fn constants_are_locked_and_the_path_is_the_reports() {
        let reactor = card();
        assert!(reactor.is_valid());
        for index in 0..reactor.constants.len() {
            let mut altered = reactor;
            altered.constants[index] = f64::from_bits(altered.constants[index].to_bits() ^ 1);
            assert!(!altered.is_valid(), "constant {index}");
        }
        let mut altered = reactor;
        altered.group_fraction[3] = 0.0026;
        assert!(!altered.is_valid());
        let mut state = start(0.0);
        let before = state;
        let mut frame = marked_frame();
        assert_eq!(step(&altered, &mut state, &mut frame, load(0.0), 0.05), Err(StepReject::InvalidSpecimen));
        assert_eq!(state, before);
        // the four residence times make the loop, and 252 parcels of 0.1 s fill it
        assert!((LOWER_PLENUM_S + GRAPHITE_S + UPPER_PLENUM_S + EXTERNAL_S - LOOP_S).abs() < 1.0e-12);
        assert!((PARCELS as f64 * PARCEL_S - LOOP_S).abs() < 1.0e-12);
        // the graphite is 65.5 in from 828 ft 0.25 in; the outlet at 835 ft 3.625 in
        assert!((GRAPHITE_TOP_IN - GRAPHITE_BOTTOM_IN - 65.5).abs() < 1.0e-12);
        assert!((GRAPHITE_BOTTOM_IN - (828.0 * 12.0 + 0.25 - (827.0 * 12.0 + 7.15))).abs() < 1.0e-9);
        assert!((OUTLET_IN - (835.0 * 12.0 + 3.625 - (827.0 * 12.0 + 7.15))).abs() < 1.0e-9);
        // the calibrated shape is centred with the 79 in effective core and lies inside the vessel
        assert!((FLUX_BOTTOM_IN + 0.5 * FLUX_HEIGHT_IN - 39.5).abs() < 1.0e-12);
        assert!(FLUX_BOTTOM_IN > HEAD_BOTTOM_IN && FLUX_BOTTOM_IN + FLUX_HEIGHT_IN < OUTLET_IN);
        let sum: f64 = GROUP_FRACTION.iter().sum();
        assert!((sum - 0.006661).abs() < 1.0e-12);
        // the shape is normalised: ∫ s = 1 and ∫ s w = 1, here by a fine midpoint sum
        let shape = reactor.shape();
        let n = 252_000;
        let h = LOOP_S / n as f64;
        let (mut first, mut second) = (0.0, 0.0);
        for k in 0..n {
            let phi = shape.phi((k as f64 + 0.5) * h);
            first += phi * shape.source_scale * h;
            second += phi * shape.source_scale * phi * shape.weight_scale * h;
        }
        assert!((first - 1.0).abs() < 1.0e-8 && (second - 1.0).abs() < 1.0e-8, "{first} {second}");
        assert_eq!(reactor.gate_dt_s(), Some(0.05));
        assert_eq!(reactor.floor_dt_s(), Some(1.0e-6));
    }

    #[test]
    fn the_starts_are_the_equilibria_and_only_card_starts_start() {
        let reactor = card();
        // static: the rods sit at the static critical position, up to the parcel quadrature
        let s0 = start(0.0);
        assert!(s0.circulation_loss_percent_dk_k.abs() < 1.0e-5, "{}", s0.circulation_loss_percent_dk_k);
        // circulating, at full speed and at half: the closed form is the equilibrium the card's own step holds
        for speed in [1.0, 0.5] {
            let s1 = start(speed);
            let mut held = s1;
            let mut frame = marked_frame();
            let steps = (504.0 / speed) as usize;
            for _ in 0..steps {
                step(&reactor, &mut held, &mut frame, load(speed), 0.05).unwrap();
            }
            assert!((held.circulation_loss_percent_dk_k - s1.circulation_loss_percent_dk_k).abs() < 2.0e-6, "{speed}");
            for g in 0..6 {
                let i0 = inventory(&s1.precursor_ws[g]);
                assert!((inventory(&held.precursor_ws[g]) / i0 - 1.0).abs() < 1.0e-5, "group {g} at {speed}");
            }
            assert_eq!(held.fuel_flow_fraction, speed);
        }
        // a slower steady flow loses less, and the loss goes to zero with the flow
        let half = start(0.5).circulation_loss_percent_dk_k;
        let crawl = start(1.0e-6).circulation_loss_percent_dk_k;
        assert!(half > 0.0 && half < reactor.steady_loss_percent_dk_k());
        assert!(crawl.abs() < 1.0e-5, "{crawl}");
        // a precursor is either inside the vessel or outside it
        let s1 = start(1.0);
        let (v, o) = inventories(&reactor, &s1);
        for g in 0..6 {
            assert!(((v[g] + o[g]) / inventory(&s1.precursor_ws[g]) - 1.0).abs() < 1.0e-12);
            assert!(o[g] > 0.0);
        }
        assert!(ReactorMsrePumpState::try_start(&reactor, 1.0_f64.next_up()).is_none());
        assert!(ReactorMsrePumpState::try_start(&reactor, (-0.0_f64).next_down()).is_none());
        assert!(ReactorMsrePumpState::try_start(&reactor, f64::NAN).is_none());
        assert!(ReactorMsrePumpState::try_start(&reactor, f64::MIN_POSITIVE).is_some());
    }

    #[test]
    fn every_refusal_is_named_and_rolls_back_state_and_frame() {
        let reactor = card();
        let fresh = start(0.0);
        let full = start(1.0);
        let refuse = |state: ReactorMsrePumpState, l: ReactorMsrePumpLoad, dt: f64, expected: StepReject| {
            let mut s = state;
            let mut frame = marked_frame();
            let before = frame.to_bytes();
            assert_eq!(step(&reactor, &mut s, &mut frame, l, dt), Err(expected));
            assert_eq!(format!("{:?}", s), format!("{:?}", state));
            assert_eq!(frame.to_bytes(), before);
        };
        refuse(fresh, load(0.0), f64::NAN, StepReject::NonFiniteInput);
        refuse(fresh, load(f64::INFINITY), 0.05, StepReject::NonFiniteInput);
        let mut bad = fresh;
        bad.precursor_ws[2][7] = f64::NAN;
        refuse(bad, load(0.0), 0.05, StepReject::NonFiniteState);
        refuse(fresh, load(0.0), 0.0, StepReject::NonPositiveDt);
        refuse(fresh, load(0.0), -0.05, StepReject::NonPositiveDt);
        refuse(full, load(1.0_f64.next_up()), 0.05, StepReject::FlowOutsideCard);
        refuse(fresh, load(-0.01), 0.05, StepReject::FlowOutsideCard);
        let mut over = full;
        over.fuel_flow_fraction = 1.0_f64.next_up();
        refuse(over, load(1.0), 0.05, StepReject::FlowOutsideCard);
        let mut negative = fresh;
        negative.precursor_ws[0][0] = -1.0e-30;
        refuse(negative, load(0.0), 0.05, StepReject::NegativeState);
        let mut moved = fresh;
        moved.precursor_ws[4][100] *= 1.0 + 1.0e-12;
        refuse(moved, load(0.0), 0.05, StepReject::InconsistentState);
        // the edges land: the pump stopped and at full speed, and a step from stopped to full in one step
        for (state, l) in [(fresh, load(0.0)), (full, load(1.0)), (fresh, load(1.0)), (full, load(0.0))] {
            let mut s = state;
            let mut frame = marked_frame();
            assert!(step(&reactor, &mut s, &mut frame, l, 0.05).is_ok());
            assert_eq!(frame.proof, [0xA5; 16]);
            assert!(s.fuel_flow_fraction >= 0.0 && s.fuel_flow_fraction <= 1.0);
        }
    }

    #[test]
    fn the_fuel_flow_follows_the_pump_with_its_lag() {
        let reactor = card();
        // the pump held at full speed from stopped: f = 1 − e^(−t / 2.55)
        let mut state = start(0.0);
        let mut frame = marked_frame();
        for _ in 0..51 {
            step(&reactor, &mut state, &mut frame, load(1.0), 0.05).unwrap();
        }
        // the first step's ramp from 0 to 1 is linear; afterwards the pump is held, so the flow at 2.55 s is
        // 1 − (1 − f(0.05)) e^(−2.5 / 2.55), f(0.05) = 1 − (2.55 / 0.05)(1 − e^(−0.05 / 2.55))
        let f005 = 1.0 - FUEL_FLOW_LAG_S / 0.05 * (1.0 - crate::math::exp(-0.05 / FUEL_FLOW_LAG_S));
        let held = 1.0 - (1.0 - f005) * crate::math::exp(-2.5 / FUEL_FLOW_LAG_S);
        assert!((state.fuel_flow_fraction - held).abs() < 1.0e-14, "{} {held}", state.fuel_flow_fraction);
        // that first step, from stopped to full speed in 0.05 s, is faster than the record: flagged
        let mut first = start(0.0);
        step(&reactor, &mut first, &mut frame, load(1.0), 0.05).unwrap();
        assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED_RUNS, FLAG_OUTSIDE_COMPARED_RUNS);
        // at the floor the flow moves by the step's share and stays on the card
        let mut state = start(0.0);
        step(&reactor, &mut state, &mut frame, load(1.0e-6 * 1.3), 1.0e-6).unwrap();
        assert!(state.fuel_flow_fraction > 0.0 && state.fuel_flow_fraction < 1.0e-12, "{}", state.fuel_flow_fraction);
    }

    #[test]
    fn a_run_closes_the_ledger_per_group_and_flags_what_was_not_compared() {
        let run = run_with(rk4, 0.0, ramp_up, 0.05, 1600);
        assert_eq!(run.end, None);
        assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT && run.largest_residual > 0.0);
        let s = run.last;
        for g in 0..6 {
            let change = inventory(&s.precursor_ws[g]) - s.inventory_anchor_ws[g];
            let books = s.born_ws[g] - s.decayed_vessel_ws[g] - s.decayed_outside_ws[g];
            assert!((change - books).abs() <= RESIDUAL_CONSERVATION_LIMIT * s.inventory_anchor_ws[g], "group {g}");
            assert!(s.decayed_outside_ws[g] > 0.0);
        }
        // the rods made up the loss: positive, and at 80 s near the steady loss (group 1 still settling)
        assert!(s.rod_compensation_percent_dk_k > 0.9 * card().steady_loss_percent_dk_k());
        // past 70 s the step lands and says so
        assert!(s.outside_compared_runs);
        let reactor = card();
        let mut state = start(0.0);
        let mut frame = marked_frame();
        step(&reactor, &mut state, &mut frame, load(0.05), 0.05).unwrap();
        assert_eq!(frame.flags, FLAG_FLOW_CHANGING | FLAG_FUEL_CIRCULATING);
        let mut state = start(1.0);
        step(&reactor, &mut state, &mut frame, load(1.0), 0.05).unwrap();
        assert_eq!(frame.flags, FLAG_FUEL_CIRCULATING);
        let mut state = start(0.0);
        step(&reactor, &mut state, &mut frame, load(0.0), 0.05).unwrap();
        assert_eq!(frame.flags, 0);
        // a rise of 1.3 per s and a fall of 0.81 per s are compared; faster lands, flagged
        for (from, to, flagged) in [(0.0, 1.3 * 0.05, false), (1.0, 1.0 - 0.81 * 0.05, false), (0.0, 0.066, true), (1.0, 0.959, true)] {
            let mut state = start(from);
            step(&reactor, &mut state, &mut frame, load(to), 0.05).unwrap();
            assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED_RUNS != 0, flagged, "{from} to {to}");
        }
        // a start at a steady speed between the two compared is flagged on every step
        let mut state = start(0.5);
        step(&reactor, &mut state, &mut frame, load(0.5), 0.05).unwrap();
        assert!(state.outside_compared_runs);
    }

    #[test]
    fn a_wrong_method_is_named_by_the_ledger() {
        for method in [euler as Method, equal_weights, leaky_source] {
            let run = run_with(method, 0.0, ramp_up, 0.05, 1400);
            assert!(
                run.end == Some(StepReject::SpontaneousGeneration) || run.end == Some(StepReject::FirstLawBreach),
                "{:?} after {} steps",
                run.end,
                run.steps
            );
        }
        // a falsified ledger in a handed-in state is not one the card wrote
        let reactor = card();
        let mut state = start(0.0);
        let mut frame = marked_frame();
        for n in 0..40 {
            step(&reactor, &mut state, &mut frame, load(ramp_up((n + 1) as f64 * 0.05)), 0.05).unwrap();
        }
        for field in 0..4 {
            let mut forged = state;
            match field {
                0 => forged.born_ws[1] *= 1.0 + 1.0e-6,
                1 => forged.decayed_outside_ws[5] += 1.0e-9,
                2 => forged.residual_conservation_error = 0.0,
                _ => forged.rod_compensation_percent_dk_k += 1.0e-6,
            }
            let before = forged;
            let mut f = frame;
            assert_eq!(step(&reactor, &mut forged, &mut f, load(state.pump_speed_fraction), 0.05), Err(StepReject::InconsistentState), "field {field}");
            assert_eq!(format!("{:?}", forged), format!("{:?}", before));
        }
    }

    #[test]
    fn the_largest_residual_at_the_ceiling_is_under_the_limit_at_the_corners() {
        let profiles: [(f64, fn(f64) -> f64); 4] = [
            (0.0, |t| if 1.3 * t < 1.0 { 1.3 * t } else { 1.0 }),
            (1.0, |t| if 1.0 - 0.81 * t > 0.0 { 1.0 - 0.81 * t } else { 0.0 }),
            (1.0, |_| 1.0),
            (1.0, |t| 0.5 + 0.5 * crate::math::cos(1.62 * t)),
        ];
        let mut largest = 0.0f64;
        for (start_speed, profile) in profiles {
            let run = run_with(rk4, start_speed, profile, 0.05, 1400);
            assert_eq!((run.end, run.steps), (None, 1400));
            largest = largest.max(run.largest_residual);
        }
        assert!(largest <= RESIDUAL_CONSERVATION_LIMIT, "{largest}");
        assert!(largest > RESIDUAL_CONSERVATION_LIMIT / 40.0, "{largest}");
    }

    /// The fuel-pump speed readings of ORNL-4233 Figs. 22 and 23 as digitised
    /// (evidence/research/reactor_msre_pump/data), in s and percent of full scale.
    const STARTUP: [(f64, f64); 14] = [
        (0.0, 0.0), (0.0810373, 9.94609), (0.218801, 22.0755), (0.251216, 25.6334), (0.356564, 38.248),
        (0.429498, 45.5256), (0.502431, 52.8032), (0.648298, 63.8005), (0.769854, 76.7385), (0.82658, 82.5606),
        (0.964344, 94.8518), (1.02917, 100.35), (1.3047, 100.35), (10.0, 100.35),
    ];
    const COASTDOWN: [(f64, f64); 20] = [
        (0.00809717, 100.0), (0.0728745, 94.7665), (0.234818, 85.1047), (0.396761, 77.0531), (0.720648, 64.9758),
        (1.02834, 56.5217), (1.53036, 44.847), (2.0, 35.0242), (2.50202, 29.5491), (3.02834, 25.4428),
        (3.55466, 22.3027), (4.04049, 19.8873), (5.02834, 15.4589), (6.04858, 12.6409), (7.02024, 9.42029),
        (8.02429, 8.45411), (9.04453, 6.19968), (10.0324, 4.1868), (11.0202, 2.81804), (12.0405, 0.161031),
    ];

    fn interpolate(points: &[(f64, f64)], full: f64, t: f64) -> f64 {
        for pair in points.windows(2) {
            let (x0, y0) = (pair[0].0, pair[0].1 / full);
            let (x1, y1) = (pair[1].0, pair[1].1 / full);
            if x0 <= t && t <= x1 {
                return y0 + (y1 - y0) * (t - x0) / (x1 - x0);
            }
        }
        f64::NAN
    }

    /// The comparison's schedules, by the rules of `analysis/compare.py`.
    fn startup_speed(t: f64) -> f64 {
        if t >= 10.0 { 1.0 } else { interpolate(&STARTUP, 100.35, t).min(1.0) }
    }

    fn coastdown_speed(t: f64) -> f64 {
        if t <= 0.00809717 {
            1.0
        } else if t >= 12.0405 {
            0.161031 / 100.0
        } else {
            interpolate(&COASTDOWN, 100.0, t).max(0.0).min(1.0)
        }
    }

    /// The card's answers on the comparison's schedules as the calibration of 2026-10-18 recorded
    /// them (calibration_2026-10-18/out/card_*.tsv), pinned so that a change to the shape, the
    /// lag, the transport or the weighting shows.
    #[test]
    fn the_comparison_runs_give_the_answers_the_calibration_recorded() {
        let reactor = card();
        let steady = reactor.steady_loss_percent_dk_k();
        assert!((steady - 0.212).abs() < 1.0e-4, "{steady}");
        assert!((steady - STEADY_PIN).abs() < 1.0e-12, "{steady}");
        for (start_speed, profile, pins) in [
            (0.0, startup_speed as fn(f64) -> f64, STARTUP_PINS),
            (1.0, coastdown_speed as fn(f64) -> f64, COASTDOWN_PINS),
        ] {
            let mut state = ReactorMsrePumpState::try_start(&reactor, start_speed).unwrap();
            let mut frame = marked_frame();
            let mut at = [f64::NAN; 1401];
            for n in 0..1400usize {
                step(&reactor, &mut state, &mut frame, load(profile((n + 1) as f64 * 0.05)), 0.05).unwrap();
                at[n + 1] = state.rod_compensation_percent_dk_k;
            }
            for (n, value) in pins {
                assert!((at[n] - value).abs() < 1.0e-9, "step {n}: {:.18} against {value}", at[n]);
            }
        }
    }

    const STEADY_PIN: f64 = 0.21198486747418477;
    const STARTUP_PINS: [(usize, f64); 2] = [(260, 0.25963240441332736), (1400, 0.2212847388238603)];
    const COASTDOWN_PINS: [(usize, f64); 2] = [(400, -0.14003500605054375), (1400, -0.19578704154648902)];
}
