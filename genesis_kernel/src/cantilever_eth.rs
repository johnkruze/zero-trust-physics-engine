#![no_implicit_prelude]

//! The ETH Zürich aluminium cantilever, first bending mode, free ring-down.
//!
//! One aluminium strip, 326 mm long, 40 mm wide and 1 mm thick by the
//! dataset's readme, alloy not stated, lying flat, clamped at one end under a
//! bolted steel plate with nothing between plate and strip, bending up and
//! down in room air on 2019-03-25. Recorded by Lai, Alzugaray, Chli and
//! Chatzi, Zenodo, doi 10.5281/zenodo.22723484, licence CC BY 4.0.
//!
//! The card's deflection is the deflection at the laser-1 station, 268 mm from
//! the clamp, in metres. The card describes a free decay of the first bending
//! mode from an envelope of 18.75 mm at that station, the largest amplitude
//! its constants were fitted on, down to rest. A run of the card was compared
//! with a ring-down it was not fitted to from 15.786 mm down to 0.0587 mm;
//! above 15.79 mm a frame carries [`FLAG_ENVELOPE_ABOVE_COMPARED`] and below
//! 0.058 mm [`FLAG_ENVELOPE_BELOW_COMPARED`]. There is no load: the beam was
//! never driven in the data.
//!
//! Where the numbers come from, and what they rest on:
//!
//! - The six constants are fitted to one recording, `test1a` of the dataset,
//!   from 0.863 s to 24 s after its start: least squares on the amplitude and
//!   the frequency of 23 blocks of eight periods. They are structure E of
//!   the fit made on that recording, `analysis/out/constants.json` in this
//!   card's research record, `resonator_eth_aluminium_cantilever`.
//! - The laser was logged in volts. The metres scale rests on one line of the
//!   authors' own script, 120 / (499 × 20) / 2 = 0.006012024 m per volt,
//!   cross-checked against the strain gauge and beam theory to 3 percent. It
//!   does not rest on a calibration record. `analysis/05_card_emulation.py`
//!   converted the fitted constants with that factor and rounded them to six
//!   figures; the decimals here are copied from it and are not recomputed. In
//!   volts they are damping 0.495302, drag 0.00476902, softening −0.0469596.
//! - There is no mass. The alloy is not stated, and the fit fixes frequencies
//!   and rates, which need none. Every equation is per unit of moving mass
//!   and every energy is in joules per kilogram.
//! - The comparison. The second ring-down of this beam, `test1b`, was cut in
//!   two; the forecast of structure E on its opened twelve seconds was made
//!   in Python outside the engine (amplitude 2.22 percent against a 5 percent
//!   line, frequency 0.0047 Hz against 0.010 Hz, timing 0.024 of a period
//!   against 0.10), and this card through the engine gave the same figures.
//!   On 2026-10-09 one run of this card was compared with the sealed twelve
//!   seconds and missed the amplitude line: 7.03 percent against 5.0, with
//!   frequency 0.0031 Hz and timing 0.021 of a period inside their lines.
//!   The result is the research record's `RESULT.md`.
//!
//! The range is what that comparison spanned. The compared run started 0.150 s
//! before the start of the ring-down, at an envelope of 15.786 mm, on the
//! model's own trajectory; its readings were scored from the start crossing,
//! 0.14873 s into the run, to the end of the last scored block, 23.469 s after
//! it. Over the run the card's envelope went from 15.786 mm at its start, the
//! largest it held, through 14.404 mm at the first turning point after the
//! start crossing, to 0.0587 mm at the end of the last scored block.
//! [`COMPARED_CEILING_M`] is the first rounded up to 15.79 mm and
//! [`AMPLITUDE_FLOOR_M`] the last rounded down to 0.058 mm; the measured swing
//! of the last scored block reached smaller amplitudes than the card's. The
//! envelopes from 14.41 to 15.79 mm are the ones the compared run passed
//! through before its readings were scored. [`AMPLITUDE_CEILING_M`], the
//! largest envelope the card takes, is 18.75 mm: 3.119 V, the largest
//! amplitude of the fitted recording, times 0.006012 m per volt is 18.7514 mm,
//! rounded down. The softening and the damping are power laws fitted on
//! amplitudes up to it; above it those laws were not fitted, and the strip's
//! own limit, its yield at the clamp, needs its alloy, which the dataset does
//! not state. Above 18.75 mm the card refuses a start and a step by name, as
//! `EnvelopeAboveCard`: the envelope is a size, so a deflection or a speed of
//! either sign past it is refused under that one name. From 15.79 mm to
//! 18.75 mm the card answers with the same equations and the same ledger, and
//! each frame there carries [`FLAG_ENVELOPE_ABOVE_COMPARED`]: the fitted laws,
//! inside the amplitudes they were fitted on and above the largest envelope
//! the compared run held. Through engine 0.11 a start or a step there was
//! refused as `EnvelopeAboveCard`. Below the
//! floor the card goes on with the same equations and the same ledger down to
//! rest, and each frame there carries [`FLAG_ENVELOPE_BELOW_COMPARED`]. Every
//! force term is a power of the deflection or of the speed, and the power of
//! zero, or of anything under the smallest normal 64-bit number, is exactly
//! zero, so the map stays finite all the way to rest; rest at zero is a state
//! that does not move. Below 0.0587 mm the fitted damping and the 8.29235 Hz
//! frequency at vanishing amplitude are carried on past where they were
//! compared: the lowest frequency read on the beam was 8.22 Hz. The run that
//! was compared is accepted from its own start; from the state it reached
//! after 595 steps, 0.14875 s, the card's map gives the same deflections and
//! speeds bit for bit, and that run lands its 94,136 steps to 23.534 s inside
//! the compared envelope and goes on landing below it. A free decay from any
//! start ends only when the caller stops it.
//!
//! The card leaves out a drive, every other mode (a line near 50.7 Hz reaches
//! 1 percent in the lasers), every other clamp and every other strip, the
//! temperature and the air currents of the room, and the difference between
//! one ring-down and the next. The softening spring is a fitted power law. It
//! is not derived from the strip's dimensions and says nothing outside the
//! fitted amplitudes. The frequency at vanishing amplitude, 8.29235 Hz, is an
//! extrapolation: the lowest frequency read was 8.22 Hz.
//!
//! ```text
//! deflection   dx/dt = v                                               m/s
//! speed        dv/dt = − c |x|^s v − d |v| v − ω0² ( x + κ sign(x) |x|^p )
//! stored       E = ½ v² + ω0² ( ½ x² + κ |x|^(p+1) / (p+1) )           J/kg
//! heat         dH/dt = c |x|^s v² + d |v|³                             J/kg per s
//! defect       E − E_anchor + H                                        J/kg
//! residual     |defect| / E_scale
//! envelope     sqrt( x² + (v / ω0)² )                                  m
//! ```
//!
//! `ω0 = 2π ·` [`SMALL_AMPLITUDE_FREQUENCY_HZ`], `κ` is [`SPRING_KAPPA`], `p`
//! is [`SPRING_POWER`], `c` is [`DAMPING_C`], `s` is [`DAMPING_POWER`], `d` is
//! [`DRAG_PER_M`]. Four forces act: a spring, a softening of that spring that
//! follows a locked power of the deflection, a damping that follows a locked
//! small power of the deflection, and an air drag that goes as the square of
//! the speed. The map is fixed-step fourth-order Runge–Kutta on the deflection,
//! the speed and the heat.
//!
//! The ledger. Multiply the speed equation by `v`: the spring and its
//! softening are the derivative of the stored energy, so
//! `dE/dt = − c |x|^s v² − d |v|³ = − dH/dt`, and `E + H` stays at the energy
//! the run started with, `E_anchor`. The heat is never negative, so the stored
//! energy only falls. `E_scale` is the stored energy at rest at
//! [`LEDGER_SCALE_AMPLITUDE_M`], 18.8 mm, the largest amplitude the constants
//! were fitted on, 0.4528039711 J/kg. The Runge–Kutta step advances
//! the heat by its own rate and never evaluates `E`; the defect compares the
//! two. The ledger names a step that creates energy (explicit Euler does), a
//! heat that was altered, a force that is not the derivative of the stored
//! energy, and a step too coarse for the corner in the damping term. It cannot
//! detect a wrong constant: a card with the wrong drag or the wrong damping
//! balances its books exactly. It is per unit mass, so it says nothing about
//! joules. It shows the arithmetic and the integrator are right. It cannot
//! show the constants match the beam.
//!
//! The envelope is measured with `ω0`, the frequency at vanishing amplitude.
//! The softened spring is weaker than `ω0² x`, so within one period the
//! envelope is not constant: it is smallest as the beam passes its rest
//! position and rises toward the turning point. At the largest swing the rise
//! is about 1 percent after the decay of that quarter period is taken off. A
//! start at zero deflection with the envelope within 1 percent of the ceiling
//! is therefore accepted by `try_phase` and refused, by name, a little under a
//! quarter of a period later, when the envelope passes the ceiling. Measured:
//! at 0.99 of the ceiling the step is refused after 107 steps of 0.25 ms; at
//! 0.98 the beam rings through and the envelope tops at 0.991 of the ceiling.
//! For the same reason, as a ring-down from above 15.79 mm passes it, frames
//! with [`FLAG_ENVELOPE_ABOVE_COMPARED`] and frames without it alternate
//! within a period: from rest at 18.75 mm, 1,149 frames carry it, the last
//! at 0.317 s.
//! The ledger is checked before the ceiling, so a step that creates energy is
//! named as that.
//!
//! The power. The card needs `|x|^s`, `|x|^p` and `|x|^(p+1)` in every step
//! and `A^(p−1)` once in `is_valid`. The crate carries its own exponential and
//! natural logarithm and no power function, so the power is carried as
//! `exp(q · log(|x|))` with [`crate::math::exp`] and [`crate::math::log`], in
//! the private function `raised`. A deflection of zero, or one smaller than
//! the smallest normal 64-bit number, gives exactly zero and never a NaN.
//! The error of this composition is the error of the logarithm multiplied by
//! the size of `q · log(|x|)`: at most `3 |q log x| + 1` units in the last
//! place by the stated bounds of the two functions. Measured against
//! correctly rounded values (Python mpmath, 50 digits) over amplitudes 5e-5 m
//! to 0.0188 m, the largest error found is 1, 3, 16 and 22 units in the last
//! place for the exponents `s`, `p − 1`, `p` and `p + 1`, a relative error
//! under 4e-15. For deflections inside a swing, 1e-12 m to 5e-5 m, it is 1, 5,
//! 23 and 55 units, a relative error under 7e-15. The reference numbers of the
//! research record were made with Python's own power function, so a run of
//! this card does not match them bit for bit. From the start of the compared
//! run the card's map agrees with them to a relative 1e-12 after 4,000 steps
//! and 2e-12 after 96,800 steps, which is the thirteen figures the starting
//! state and the reference numbers are printed with.
//!
//! [`CantileverEth::gate_dt_s`] is the step ceiling, 0.25 ms, about 482 steps
//! per period. It is set by measurement, not by a formula. The damping term
//! `|x|^0.06` has a sharp corner at zero deflection, which the beam crosses
//! twice a period, so the error falls more slowly with the step than
//! fourth-order Runge–Kutta usually gives. Over the 24 s of the compared run,
//! on the card's map from its start, the largest ledger defect divided by its
//! scale is 7.0e-7 at 0.5 ms, 2.0e-7 at 0.384 ms, 5.6e-8 at 0.25 ms and
//! 1.9e-8 at 0.125 ms, against a limit of 1e-6. At 0.25 ms the ledger stands
//! at an eighteenth of its limit; through the card's own step from the start
//! of the scored ring-down to the compared floor, 0.058 mm, it reaches 7.4e-8;
//! over 100 s from rest at 15.79 mm, 7.9e-8, and from rest at the ceiling,
//! 18.75 mm, 1.04e-7, 6.3e-8 of it above 15.79 mm. Below it the scale of
//! the ledger is the same energy, so the gate is too: a ring-down from 0.1 mm
//! over 200 s to 4.6e-13 m leaves 4.6e-12, and explicit Euler from 0.05 mm is
//! named as created energy after 781 steps. Ten
//! steps make one frame at 400 per second, the rate at which the reading
//! procedure of the validation protocol was checked. After 1 s the state is
//! 7.2e-8 m from a tight-tolerance reference in the plane of deflection and
//! speed / ω0, 0.0007 percent of the envelope there; the research record,
//! reading whole periods, puts the same step at 0.003 percent in amplitude
//! and 0.000004 Hz from that reference.
//! [`CantileverEth::floor_dt_s`] is the ceiling divided by 2^18,
//! 9.536743e-10 s. It is set where the deflection moves least, at a turning
//! point at the amplitude ceiling: from rest at ±18.75 mm one step at the
//! floor moves the deflection by six units in the last place, toward rest
//! (five at ±15.79 mm), and no step from one to four floors, four steps each,
//! is refused there.
//! At a quarter of this floor the deflection moves by less than a unit in the
//! last place while the speed grows, and 153 of 2,000 steps of one to four of
//! that quarter were refused as `EnvelopeAboveCard` by rounding alone. At
//! 24.2 s the floor still moves the clock. The public step applies neither.
//! The gateway refuses a step outside them.
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes time, deflection, speed, the
//! envelope, the kinetic energy, the spring energy, the heat so far, the
//! margin to the largest envelope compared, 15.79 mm, below zero above it,
//! and the conservation residual into [`LastStateFrame64`], and the two
//! flags. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Frequency of the first bending mode at vanishing amplitude, Hz. Fitted to
/// recording `test1a`. An extrapolation: the lowest frequency read was 8.22 Hz.
pub const SMALL_AMPLITUDE_FREQUENCY_HZ: f64 = 8.29235;
/// Softening of the spring `κ`, per m^(p−1). Fitted to `test1a`. Negative
/// weakens the spring.
pub const SPRING_KAPPA: f64 = -0.183144;
/// The power `p` of the deflection in the softening term. Fitted to `test1a`.
pub const SPRING_POWER: f64 = 1.26613;
/// Damping coefficient `c`, 1/s per m^s. Fitted to `test1a`.
pub const DAMPING_C: f64 = 0.673442;
/// The power `s` of the deflection in the damping term. Fitted to `test1a`.
pub const DAMPING_POWER: f64 = 0.0600771;
/// Air drag `d`: deceleration per speed squared, 1/m. Fitted to `test1a`.
pub const DRAG_PER_M: f64 = 0.793247;
/// Largest deflection envelope the card accepts, m: 3.119 V, the largest
/// amplitude of the fitted recording, times 0.006012 m per volt, 18.7514 mm,
/// rounded down to 18.75 mm. The softening and the damping are power laws
/// fitted on amplitudes up to it; above it they were not fitted, and the
/// strip's own limit, its yield at the clamp, needs its alloy, which the
/// dataset does not state. Above it a start and a step are refused as
/// [`StepReject::EnvelopeAboveCard`]. See the header.
pub const AMPLITUDE_CEILING_M: f64 = 0.01875;
/// Largest deflection envelope the card was compared at, m: the largest
/// envelope the card held on the compared run, 15.786 mm at its start,
/// rounded up to 15.79 mm. The readings were scored from 14.404 mm, at the
/// first turning point after the start crossing; the envelopes above it are
/// the ones the compared run passed through before that. Above it, up to
/// [`AMPLITUDE_CEILING_M`], a frame carries [`FLAG_ENVELOPE_ABOVE_COMPARED`].
/// Through engine 0.11 it was the card's ceiling. See the header.
pub const COMPARED_CEILING_M: f64 = 0.01579;
/// Smallest deflection envelope the card was compared at, m: the smallest
/// envelope the card held on the compared run up to the end of the last
/// scored block, 0.0587 mm, rounded down to 0.058 mm. Below it a frame
/// carries [`FLAG_ENVELOPE_BELOW_COMPARED`]. See the header.
pub const AMPLITUDE_FLOOR_M: f64 = 5.8e-5;
/// The amplitude whose stored energy at rest is the ledger's scale, m:
/// 3.119 V × 0.006012 m/V = 18.75 mm, the largest amplitude the constants
/// were fitted on, taken up to 18.8 mm. It sets the scale only; the card
/// accepts no envelope above [`AMPLITUDE_CEILING_M`], the same amplitude
/// rounded down to 18.75 mm.
pub const LEDGER_SCALE_AMPLITUDE_M: f64 = 0.0188;
/// Step ceiling, s: 0.25 ms, about 482 steps per period. Set by measurement;
/// see the header.
pub const GATE_DT_S: f64 = 2.5e-4;
/// Step ceiling divided by step floor: 2^18.
pub const GATE_OVER_FLOOR: f64 = 262_144.0;
/// `|E − E_anchor + H| / E_scale` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;

/// The envelope on this frame is below 0.058 mm, the smallest the card was
/// compared at on the held-back ring-down: the same equations and the same
/// ledger, with the fitted damping and the small-amplitude frequency carried
/// on below where they were compared.
pub const FLAG_ENVELOPE_BELOW_COMPARED: u64 = 1 << 0;
/// The envelope on this frame is above 15.79 mm, the largest the card held on
/// the run compared with the held-back ring-down, and at or below 18.75 mm,
/// the largest amplitude the constants were fitted on: the same equations and
/// the same ledger, with the fitted laws inside the amplitudes they were
/// fitted on and above the largest envelope compared. The state holds no
/// boolean of its own for it; the frame's flags alone carry it. Through
/// engine 0.11 such a start or step was refused as
/// [`StepReject::EnvelopeAboveCard`].
pub const FLAG_ENVELOPE_ABOVE_COMPARED: u64 = 1 << 1;

/// Why a step did not advance the beam or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or a stage of the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this beam's.
    InvalidBeam,
    /// The deflection envelope, a size, is above 18.75 mm at the start of the step or where the trial step lands: a deflection or a speed of either sign larger than the card takes. 18.75 mm is 3.119 V × 0.006012 m/V, the largest amplitude the softening and the damping were fitted on; above it those laws were not fitted, and the strip's own limit, its yield at the clamp, needs its alloy, which the dataset does not state.
    EnvelopeAboveCard,
    /// The trial step created mechanical energy: the stored energy plus the
    /// heat rose above the energy the run started with.
    SpontaneousGeneration,
    /// The trial step destroyed mechanical energy: the stored energy plus the
    /// heat fell below the energy the run started with.
    FirstLawBreach,
}

/// The beam rings free. The load is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CantileverLoad {}

/// The locked beam. Every field is a recorded copy of a crate constant and
/// none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CantileverEth {
    /// Recorded copy of [`SMALL_AMPLITUDE_FREQUENCY_HZ`].
    pub small_amplitude_frequency_hz: f64,
    /// Recorded copy of [`SPRING_KAPPA`].
    pub spring_kappa: f64,
    /// Recorded copy of [`SPRING_POWER`].
    pub spring_power: f64,
    /// Recorded copy of [`DAMPING_C`].
    pub damping_c: f64,
    /// Recorded copy of [`DAMPING_POWER`].
    pub damping_power: f64,
    /// Recorded copy of [`DRAG_PER_M`].
    pub drag_per_m: f64,
    /// Recorded copy of [`AMPLITUDE_CEILING_M`].
    pub amplitude_ceiling_m: f64,
    /// Recorded copy of [`COMPARED_CEILING_M`].
    pub compared_ceiling_m: f64,
    /// Recorded copy of [`AMPLITUDE_FLOOR_M`].
    pub amplitude_floor_m: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`GATE_OVER_FLOOR`].
    pub gate_over_floor: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
}

impl CantileverEth {
    /// Card: the first bending mode of the ETH Zürich aluminium strip, clamped
    /// with no rubber, in air. The constructor takes no arguments.
    #[must_use]
    pub fn first_bending_mode() -> Self {
        let beam = Self {
            small_amplitude_frequency_hz: SMALL_AMPLITUDE_FREQUENCY_HZ,
            spring_kappa: SPRING_KAPPA,
            spring_power: SPRING_POWER,
            damping_c: DAMPING_C,
            damping_power: DAMPING_POWER,
            drag_per_m: DRAG_PER_M,
            amplitude_ceiling_m: AMPLITUDE_CEILING_M,
            compared_ceiling_m: COMPARED_CEILING_M,
            amplitude_floor_m: AMPLITUDE_FLOOR_M,
            gate_dt_s: GATE_DT_S,
            gate_over_floor: GATE_OVER_FLOOR,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if beam.is_valid() {
            beam
        } else {
            ::core::panic!("cantilever beam failed is_valid");
        }
    }

    /// Every recorded copy equals its crate constant bit for bit, the two
    /// powers are in their ranges (`1 < p < 3`, `0 ≤ s < 1`), the floor, the
    /// compared ceiling and the ceiling are in that order, the spring
    /// softens, and it still restores at the amplitude ceiling.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.small_amplitude_frequency_hz.to_bits() == SMALL_AMPLITUDE_FREQUENCY_HZ.to_bits()
            && self.spring_kappa.to_bits() == SPRING_KAPPA.to_bits()
            && self.spring_power.to_bits() == SPRING_POWER.to_bits()
            && self.damping_c.to_bits() == DAMPING_C.to_bits()
            && self.damping_power.to_bits() == DAMPING_POWER.to_bits()
            && self.drag_per_m.to_bits() == DRAG_PER_M.to_bits()
            && self.amplitude_ceiling_m.to_bits() == AMPLITUDE_CEILING_M.to_bits()
            && self.compared_ceiling_m.to_bits() == COMPARED_CEILING_M.to_bits()
            && self.amplitude_floor_m.to_bits() == AMPLITUDE_FLOOR_M.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.small_amplitude_frequency_hz > 0.0
            && self.spring_kappa < 0.0
            && self.spring_power > 1.0
            && self.spring_power < 3.0
            && self.damping_c > 0.0
            && self.damping_power >= 0.0
            && self.damping_power < 1.0
            && self.drag_per_m > 0.0
            && self.amplitude_floor_m > 0.0
            && self.amplitude_ceiling_m > self.compared_ceiling_m
            && self.compared_ceiling_m > self.amplitude_floor_m
            && self.gate_dt_s > 0.0
            && self.gate_over_floor > 1.0
            && self.residual_conservation_limit > 0.0
            && self.spring_share_at_ceiling() > 0.0
            && self.ledger_scale_j_per_kg().is_finite()
            && self.ledger_scale_j_per_kg() > 0.0
    }

    /// `ω0 = 2π ·` the frequency at vanishing amplitude, rad/s.
    #[must_use]
    pub fn omega0_rad_s(&self) -> f64 {
        2.0 * core::f64::consts::PI * self.small_amplitude_frequency_hz
    }

    /// `1 + κ A^(p−1)` at the amplitude ceiling `A`: the softened spring force
    /// as a share of the plain spring force there. 0.936 for this beam.
    #[must_use]
    pub fn spring_share_at_ceiling(&self) -> f64 {
        1.0 + self.spring_kappa * raised(self.amplitude_ceiling_m, self.spring_power - 1.0)
    }

    /// Energy of the softened spring, `ω0² ( ½ x² + κ |x|^(p+1) / (p+1) )`, J/kg.
    #[must_use]
    pub fn potential_j_per_kg(&self, deflection_m: f64) -> f64 {
        let omega0 = self.omega0_rad_s();
        omega0
            * omega0
            * (0.5 * deflection_m * deflection_m
                + self.spring_kappa * raised(deflection_m.abs(), self.spring_power + 1.0)
                    / (self.spring_power + 1.0))
    }

    /// Stored energy `E = ½ v² +` the spring's energy, J/kg.
    #[must_use]
    pub fn stored_j_per_kg(&self, deflection_m: f64, velocity_m_s: f64) -> f64 {
        0.5 * velocity_m_s * velocity_m_s + self.potential_j_per_kg(deflection_m)
    }

    /// `E_scale`: the stored energy at rest at [`LEDGER_SCALE_AMPLITUDE_M`],
    /// 18.8 mm, J/kg. 0.4528039711 for this beam.
    #[must_use]
    pub fn ledger_scale_j_per_kg(&self) -> f64 {
        self.potential_j_per_kg(LEDGER_SCALE_AMPLITUDE_M)
    }

    /// `sqrt( x² + (v / ω0)² )`, m.
    #[must_use]
    pub fn envelope_m(&self, deflection_m: f64, velocity_m_s: f64) -> f64 {
        crate::math::hypot(deflection_m, velocity_m_s / self.omega0_rad_s())
    }

    /// Largest step the card stands behind: [`GATE_DT_S`], 0.25 ms. Not
    /// applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.gate_dt_s.is_finite() && self.gate_dt_s > 0.0 {
            Some(self.gate_dt_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^18, 9.536743e-10 s. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) if self.gate_over_floor.is_finite() && self.gate_over_floor > 0.0 => {
                Some(gate_dt_s / self.gate_over_floor)
            }
            _ => None,
        }
    }
}

/// `size` raised to the power `exponent`, as `exp(exponent · log(size))` with
/// the crate's own exponential and logarithm. `size` is a magnitude: zero or
/// positive. Zero, and anything smaller than the smallest normal 64-bit
/// number, gives exactly zero and never a NaN. The error is stated and
/// measured in the header and in the tests.
fn raised(size: f64, exponent: f64) -> f64 {
    if size < f64::MIN_POSITIVE {
        return 0.0;
    }
    crate::math::exp(exponent * crate::math::log(size))
}

/// The ringing beam. The ledger fields are part of the state; write them only
/// through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CantileverState {
    /// Time since the start of the run.
    pub t_s: f64,
    /// Deflection at the laser-1 station, 268 mm from the clamp, `x`.
    pub deflection_m: f64,
    /// Speed of that deflection, signed, `v`.
    pub velocity_m_s: f64,
    /// `sqrt( x² + (v / ω0)² )`.
    pub envelope_m: f64,
    /// `½ v²`.
    pub kinetic_j_per_kg: f64,
    /// Energy of the softened spring.
    pub potential_j_per_kg: f64,
    /// `E`, kinetic plus spring energy.
    pub stored_j_per_kg: f64,
    /// `E` at the state the ledger started from.
    pub energy_anchor_j_per_kg: f64,
    /// `H`, the heat lost to the damping and the air drag since the anchor. Never falls.
    pub heat_j_per_kg: f64,
    /// Signed `E − E_anchor + H`. Positive is created energy.
    pub energy_defect_j_per_kg: f64,
    /// `|energy_defect_j_per_kg| / E_scale`.
    pub residual_conservation_error: f64,
    /// The envelope is below 0.058 mm, the smallest the card was compared
    /// at: see [`FLAG_ENVELOPE_BELOW_COMPARED`].
    pub envelope_below_compared: bool,
}

impl CantileverState {
    /// Anchor the ledger at this deflection and speed, time 0. `None` for a
    /// value that is not finite or an envelope above [`AMPLITUDE_CEILING_M`],
    /// 18.75 mm.
    /// Rest at zero is a state.
    #[must_use]
    pub fn try_phase(beam: &CantileverEth, deflection_m: f64, velocity_m_s: f64) -> Option<Self> {
        if !beam.is_valid() || !deflection_m.is_finite() || !velocity_m_s.is_finite() {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            deflection_m,
            velocity_m_s,
            envelope_m: 0.0,
            kinetic_j_per_kg: 0.0,
            potential_j_per_kg: 0.0,
            stored_j_per_kg: 0.0,
            energy_anchor_j_per_kg: 0.0,
            heat_j_per_kg: 0.0,
            energy_defect_j_per_kg: 0.0,
            residual_conservation_error: 0.0,
            envelope_below_compared: false,
        };
        refresh(beam, &mut state);
        state.energy_anchor_j_per_kg = state.stored_j_per_kg;
        if !state_is_finite(&state) || !(state.envelope_m <= beam.amplitude_ceiling_m) {
            return None;
        }
        Some(state)
    }
}

/// Advance one Runge–Kutta step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were. A stage of the
/// step that is not finite is [`StepReject::NonFiniteState`].
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    beam: &CantileverEth,
    state: &mut CantileverState,
    frame: &mut LastStateFrame64,
    load: CantileverLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(beam, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("cantilever_eth", exceedances(beam, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    beam: &CantileverEth,
    state: &mut CantileverState,
    frame: &mut LastStateFrame64,
    load: CantileverLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(beam, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    beam: &CantileverEth,
    state: &mut CantileverState,
    frame: &mut LastStateFrame64,
    load: CantileverLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(beam, state, load, dt_s)?;
    let advanced = match rk4(beam, state_vector(state), dt_s) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(beam, state, frame, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut CantileverState) {
    state.t_s = normal_or_zero(state.t_s);
    state.deflection_m = normal_or_zero(state.deflection_m);
    state.velocity_m_s = normal_or_zero(state.velocity_m_s);
    state.envelope_m = normal_or_zero(state.envelope_m);
    state.kinetic_j_per_kg = normal_or_zero(state.kinetic_j_per_kg);
    state.potential_j_per_kg = normal_or_zero(state.potential_j_per_kg);
    state.stored_j_per_kg = normal_or_zero(state.stored_j_per_kg);
    state.energy_anchor_j_per_kg = normal_or_zero(state.energy_anchor_j_per_kg);
    state.heat_j_per_kg = normal_or_zero(state.heat_j_per_kg);
    state.energy_defect_j_per_kg = normal_or_zero(state.energy_defect_j_per_kg);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

fn validate(
    beam: &CantileverEth,
    state: &CantileverState,
    _load: CantileverLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !beam.is_valid() {
        return Err(StepReject::InvalidBeam);
    }
    // The envelope is worked out again from the deflection and the speed; the
    // recorded copy in the state is not trusted.
    let envelope_m = beam.envelope_m(state.deflection_m, state.velocity_m_s);
    if !(envelope_m <= beam.amplitude_ceiling_m) {
        return Err(StepReject::EnvelopeAboveCard);
    }
    Ok(())
}

fn commit(
    beam: &CantileverEth,
    state: &mut CantileverState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
    advanced: [f64; 3],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite3(advanced) {
        return Err(StepReject::NonFiniteState);
    }
    state.t_s += dt_s;
    state.deflection_m = advanced[0];
    state.velocity_m_s = advanced[1];
    state.heat_j_per_kg = advanced[2];
    refresh(beam, state);
    let scale = beam.ledger_scale_j_per_kg();
    if !(scale.is_finite() && scale > 0.0) {
        *state = saved;
        return Err(StepReject::InvalidBeam);
    }
    state.energy_defect_j_per_kg =
        state.stored_j_per_kg - state.energy_anchor_j_per_kg + state.heat_j_per_kg;
    state.residual_conservation_error = state.energy_defect_j_per_kg.abs() / scale;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    let gate = beam.residual_conservation_limit * scale;
    if state.energy_defect_j_per_kg > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j_per_kg < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    if !(state.envelope_m <= beam.amplitude_ceiling_m) {
        *state = saved;
        return Err(StepReject::EnvelopeAboveCard);
    }
    publish(beam, state, frame);
    Ok(())
}

/// The energies and the envelope of the deflection and the speed now in the
/// state.
fn refresh(beam: &CantileverEth, state: &mut CantileverState) {
    state.kinetic_j_per_kg = 0.5 * state.velocity_m_s * state.velocity_m_s;
    state.potential_j_per_kg = beam.potential_j_per_kg(state.deflection_m);
    state.stored_j_per_kg = state.kinetic_j_per_kg + state.potential_j_per_kg;
    state.envelope_m = beam.envelope_m(state.deflection_m, state.velocity_m_s);
    state.envelope_below_compared = state.envelope_m < beam.amplitude_floor_m;
}

fn publish(beam: &CantileverEth, state: &CantileverState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.deflection_m as f32;
    frame.pos[1] = state.velocity_m_s as f32;
    frame.pos[2] = state.envelope_m as f32;
    frame.vel[0] = state.kinetic_j_per_kg as f32;
    frame.vel[1] = state.potential_j_per_kg as f32;
    frame.vel[2] = state.heat_j_per_kg as f32;
    // The margin to the largest envelope compared, 15.79 mm, as through
    // engine 0.11, when it was the ceiling: below zero above it.
    frame.force_torque = (beam.compared_ceiling_m - state.envelope_m) as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.envelope_below_compared {
        flags |= FLAG_ENVELOPE_BELOW_COMPARED;
    }
    if state.envelope_m > beam.compared_ceiling_m {
        flags |= FLAG_ENVELOPE_ABOVE_COMPARED;
    }
    frame.flags = flags;
}

/// The three variables in the order of the Runge–Kutta step: deflection,
/// speed, heat.
fn state_vector(state: &CantileverState) -> [f64; 3] {
    [state.deflection_m, state.velocity_m_s, state.heat_j_per_kg]
}

/// The right-hand side of the three equations at `y`. At zero deflection the
/// damping that follows the deflection and the softening are both exactly
/// zero. `None` when the point or a rate is not finite.
fn rates(beam: &CantileverEth, y: [f64; 3]) -> Option<[f64; 3]> {
    let deflection_m = y[0];
    let velocity_m_s = y[1];
    if !(deflection_m.is_finite() && velocity_m_s.is_finite()) {
        return None;
    }
    let size_m = deflection_m.abs();
    let speed_m_s = velocity_m_s.abs();
    let omega0 = beam.omega0_rad_s();
    let damping_per_s = beam.damping_c * raised(size_m, beam.damping_power);
    let softening_m = beam.spring_kappa * raised(size_m, beam.spring_power);
    let spring_m = if deflection_m >= 0.0 {
        deflection_m + softening_m
    } else {
        deflection_m - softening_m
    };
    let acceleration_m_s2 = -damping_per_s * velocity_m_s
        - beam.drag_per_m * speed_m_s * velocity_m_s
        - omega0 * omega0 * spring_m;
    let heat_rate_w_per_kg = damping_per_s * velocity_m_s * velocity_m_s
        + beam.drag_per_m * (speed_m_s * speed_m_s * speed_m_s);
    let out = [velocity_m_s, acceleration_m_s2, heat_rate_w_per_kg];
    if finite3(out) { Some(out) } else { None }
}

fn advance(y: [f64; 3], scale: f64, k: [f64; 3]) -> [f64; 3] {
    [y[0] + scale * k[0], y[1] + scale * k[1], y[2] + scale * k[2]]
}

/// Classical fourth-order Runge–Kutta on deflection, speed and heat.
fn rk4(beam: &CantileverEth, y: [f64; 3], dt_s: f64) -> Option<[f64; 3]> {
    let k1 = rates(beam, y)?;
    let k2 = rates(beam, advance(y, 0.5 * dt_s, k1))?;
    let k3 = rates(beam, advance(y, 0.5 * dt_s, k2))?;
    let k4 = rates(beam, advance(y, dt_s, k3))?;
    let mut next = y;
    let mut index = 0;
    while index < 3 {
        next[index] =
            y[index] + dt_s / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    if finite3(next) { Some(next) } else { None }
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn state_is_finite(state: &CantileverState) -> bool {
    state.t_s.is_finite()
        && state.deflection_m.is_finite()
        && state.velocity_m_s.is_finite()
        && state.envelope_m.is_finite()
        && state.kinetic_j_per_kg.is_finite()
        && state.potential_j_per_kg.is_finite()
        && state.stored_j_per_kg.is_finite()
        && state.energy_anchor_j_per_kg.is_finite()
        && state.heat_j_per_kg.is_finite()
        && state.energy_defect_j_per_kg.is_finite()
        && state.residual_conservation_error.is_finite()
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    BELOW_KIND,
    ABOVE_KIND,
];

const BELOW_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_ENVELOPE_BELOW_COMPARED",
    FLAG_ENVELOPE_BELOW_COMPARED,
    "envelope_m",
    "m",
    Side::Below,
    AMPLITUDE_FLOOR_M,
    "the smallest envelope the card was compared at on the held-back ring-down, 0.058 mm",
);
const ABOVE_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_ENVELOPE_ABOVE_COMPARED",
    FLAG_ENVELOPE_ABOVE_COMPARED,
    "envelope_m",
    "m",
    Side::Above,
    COMPARED_CEILING_M,
    "the largest envelope the compared run held, 15.79 mm; the constants were fitted to 18.75 mm",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
pub fn exceedances(
    beam: &CantileverEth,
    before: &CantileverState,
    after: &CantileverState,
    load: CantileverLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, load, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.envelope_below_compared, Exceedance::at(BELOW_KIND, beam.amplitude_floor_m, after.envelope_m));
    records.push_if(after.envelope_m > beam.compared_ceiling_m, Exceedance::at(ABOVE_KIND, beam.compared_ceiling_m, after.envelope_m));
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::println;
    use std::vec::Vec;

    use super::{
        AMPLITUDE_CEILING_M, AMPLITUDE_FLOOR_M, COMPARED_CEILING_M, CantileverEth, CantileverLoad,
        CantileverState, DAMPING_C, DAMPING_POWER, DRAG_PER_M, FLAG_ENVELOPE_ABOVE_COMPARED,
        FLAG_ENVELOPE_BELOW_COMPARED, GATE_DT_S, GATE_OVER_FLOOR,
        LEDGER_SCALE_AMPLITUDE_M, RESIDUAL_CONSERVATION_LIMIT, SMALL_AMPLITUDE_FREQUENCY_HZ,
        SPRING_KAPPA, SPRING_POWER, StepReject, advance, commit, raised, rates, rk4, state_vector,
        step, validate,
    };
    use crate::frame::LastStateFrame64;

    const NO_LOAD: CantileverLoad = CantileverLoad {};

    /// The start of the compared run: 0.150 s before the start of the
    /// held-back ring-down, on the model's own trajectory. Envelope 15.786 mm,
    /// the largest the compared run held, just under the card's ceiling. `card_spec_draft.md`, section 8,
    /// test 2.
    const START_DEFLECTION_M: f64 = -1.526315579382e-2;
    const START_VELOCITY_M_S: f64 = 2.099361936006e-1;

    /// The start of the scored ring-down on the compared run: the state after
    /// 595 steps of 0.25 ms, 0.14875 s, the first step after the reading's
    /// start crossing at 0.14873 s. Envelope 14.257 mm. The card's map from
    /// here gives the compared run's deflections and speeds bit for bit.
    const SCORED_START_STEPS: u64 = 595;
    const SCORED_START_DEFLECTION_M: f64 = -0.00104914043915228;
    const SCORED_START_VELOCITY_M_S: f64 = 0.7408030982848752;

    /// Correctly rounded `|x|^q` for the test of the power carried as
    /// `exp(q · log(|x|))`. Each row is the bit pattern of `x`, then the bit
    /// patterns of `x^s`, `x^(p−1)`, `x^p` and `x^(p+1)` rounded to the nearest
    /// 64-bit number, with `s = 0.0600771` and `p = 1.26613`, each exponent the
    /// 64-bit number that expression gives. Computed outside Rust from the
    /// exact 64-bit arguments with Python mpmath 1.3.0 at 50 digits and checked
    /// at 80 digits. `x` is 160 amplitudes of the card's range,
    /// `5e-5 · (0.0188 / 5e-5)^(i / 159)` for `i = 0..159` in 64-bit Python,
    /// with the first and the last set to 5e-5 and 0.0188 exactly.
    const POWER_IN_RANGE: [[u64; 5]; 160] = [
        [0x3f0a36e2eb1c432d, 0x3fe1a685fcb679b4, 0x3fb25948b3b199dd, 0x3ece1007b421c0b2, 0x3de8a0967d72136a],
        [0x3f0b35e17ab994e7, 0x3fe1b0a885ae0241, 0x3fb28822ba94606f, 0x3ecf8422b671c958, 0x3deacc8e3560cc0f],
        [0x3f0c3e90701c0e86, 0x3fe1bad0e06d934f, 0x3fb2b77462f7b726, 0x3ed0851dc336bdee, 0x3ded298d0ac0e5b6],
        [0x3f0d514e0abebf09, 0x3fe1c4ff104c9b7e, 0x3fb2e73ede52a829, 0x3ed151986a9fd90c, 0x3defbbe5ce460169],
        [0x3f0e6e7c1ee312c9, 0x3fe1cf3318a4749b, 0x3fb31783612836be, 0x3ed227f60c3e4b36, 0x3df1442675a438e4],
        [0x3f0f968038668979, 0x3fe1d96cfcd064c0, 0x3fb34843230f26e2, 0x3ed308b107b3cf97, 0x3df2c9f0819a9e0e],
        [0x3f1064e1df75a27b, 0x3fe1e3acc02d9f6c, 0x3fb3797f5eb9d8b9, 0x3ed3f449a75593c8, 0x3df4721a25a6a351],
        [0x3f11045a0db02896, 0x3fe1edf2661b469d, 0x3fb3ab3951fe380f, 0x3ed4eb466968b441, 0x3df63fab5eeb24ec],
        [0x3f11a9e16f7b811b, 0x3fe1f83df1fa6bed, 0x3fb3dd723dddc01f, 0x3ed5ee344ce93793, 0x3df835f098b176f3],
        [0x3f1255b2f5c9a408, 0x3fe2028f672e11b2, 0x3fb4102b668d93cd, 0x3ed6fda722075cb2, 0x3dfa5880b53e3767],
        [0x3f13080bcee2af14, 0x3fe20ce6c91b2c14, 0x3fb44366137eaa92, 0x3ed81a39de892f41, 0x3dfcab439ee118f3],
        [0x3f13c12b7c2de476, 0x3fe217441b28a232, 0x3fb477238f66123b, 0x3ed9448ef6408d29, 0x3dff32796d4315a7],
        [0x3f148153e8ce92ad, 0x3fe221a760bf4f3c, 0x3fb4ab65284545d0, 0x3eda7d50b7c81da8, 0x3e00f9611602d747],
        [0x3f1548c9811cf263, 0x3fe22c109d4a0395, 0x3fb4e02c2f7299b2, 0x3edbc531adbc2a49, 0x3e02789328767d8e],
        [0x3f1617d34b035635, 0x3fe2367fd43585ef, 0x3fb51579f9a1bd4c, 0x3edd1ced04a6da30, 0x3e04198ff887b941],
        [0x3f16eebaff485937, 0x3fe240f508f09470, 0x3fb54b4fdeec5270, 0x3ede8546f5d9feb4, 0x3e05df5262f1e7e3],
        [0x3f17cdcd23cf0d8e, 0x3fe24b703eebe5d1, 0x3fb581af3ada9aac, 0x3edfff0d3773709a, 0x3e07cd188a40dd93],
        [0x3f18b55926d68401, 0x3fe255f1799a2a7d, 0x3fb5b8996c6c3ac0, 0x3ee0c58bb8e5f816, 0x3e09e669c583da8a],
        [0x3f19a5b17b42605f, 0x3fe26078bc700dba, 0x3fb5f00fd621147c, 0x3ee19523dd49c90b, 0x3e0c2f1d14ed87da],
        [0x3f1a9f2bb5f68ceb, 0x3fe26b060ae436c5, 0x3fb62813de023731, 0x3ee26ec58af10038, 0x3e0eab60283064dd],
        [0x3f1ba220ac5080f3, 0x3fe27599686f49f9, 0x3fb660a6edaae6f9, 0x3ee352ecfec21ec9, 0x3e10afdf81b8d680],
        [0x3f1caeec93c8f48c, 0x3fe28032d88be9f4, 0x3fb699ca7251bb0a, 0x3ee4421c776a9313, 0x3e12289628676fa9],
        [0x3f1dc5ef22c944b3, 0x3fe28ad25eb6b8bc, 0x3fb6d37fdcd1d357, 0x3ee53cdc7fb8d5da, 0x3e13c28536d6e3c7],
        [0x3f1ee78bb2c038f1, 0x3fe29577fe6e58e0, 0x3fb70dc8a1b425ad, 0x3ee643bc3c8ed5e6, 0x3e15809aa2333bc4],
        [0x3f200a14b1c12694, 0x3fe2a023bb336ea7, 0x3fb748a63938e296, 0x3ee75751be9930fd, 0x3e17660682263cd9],
        [0x3f20a619a0010c1b, 0x3fe2aad59888a12e, 0x3fb7841a1f60f236, 0x3ee8783a57f9e054, 0x3e197640e5d9abb9],
        [0x3f21480c3236f170, 0x3fe2b58d99f29b95, 0x3fb7c025d3f78962, 0x3ee9a71af6173ff7, 0x3e1bb5102ca2a07c],
        [0x3f21f02612e000a5, 0x3fe2c04bc2f80e26, 0x3fb7fccada9bd732, 0x3eeae4a07fc2b5b8, 0x3e1e268fedf22f94],
        [0x3f229ea31d6802ce, 0x3fe2cb101721af7e, 0x3fb83a0abacacb40, 0x3eec318037ecb729, 0x3e20679c3e96e439],
        [0x3f2353c17379b5a7, 0x3fe2d5da99fa3db6, 0x3fb877e6ffe8f4ec, 0x3eed8e78251e8717, 0x3e21d9f38b977188],
        [0x3f240fc1931e7469, 0x3fe2e0ab4f0e7f8e, 0x3fb8b661394c7bc1, 0x3eeefc4f7df4bc91, 0x3e236cf3642ffaad],
        [0x3f24d2e66db5148c, 0x3fe2eb8239ed4598, 0x3fb8f57afa473163, 0x3ef03deb8d6c405d, 0x3e25237d0ddf343c],
        [0x3f259d757fc823c9, 0x3fe2f65f5e276b62, 0x3fb93535da30bd27, 0x3ef106f4f69c377f, 0x3e2700b2d23c667e],
        [0x3f266fb6e9cc0431, 0x3fe30142bf4fd8a5, 0x3fb975937470e1a2, 0x3ef1d9b6be42937a, 0x3e2907fdbab83e96],
        [0x3f2749f589cdb5e3, 0x3fe30c2c60fb8270, 0x3fb9b6956889dc82, 0x3ef2b6a9349c74d8, 0x3e2b3d13cdcc894d],
        [0x3f282c7f161b735f, 0x3fe3171c46c16c57, 0x3fb9f83d5a22e0cd, 0x3ef39e4a7b1a878c, 0x3e2da3fed904d467],
        [0x3f2917a438f09eb4, 0x3fe32212743aa9a0, 0x3fba3a8cf112abf4, 0x3ef4911ecc61df1d, 0x3e302091ea25d06a],
        [0x3f2a0bb8ad2ed9dc, 0x3fe32d0eed025e73, 0x3fba7d85d96a35e2, 0x3ef58fb0c7c80ed6, 0x3e318ca576001aa2],
        [0x3f2b09135c2e849c, 0x3fe33811b4b5c107, 0x3fbac129c37f7c5a, 0x3ef69a91c0759578, 0x3e3318d42045205c],
        [0x3f2c100e7cb13da5, 0x3fe3431acef41ad9, 0x3fbb057a63f869e1, 0x3ef7b25a105bc935, 0x3e34c7f2b5b64242],
        [0x3f2d2107b3016bfb, 0x3fe34e2a3f5ec9d5, 0x3fbb4a7973d5d87d, 0x3ef8d7a96f2d9fda, 0x3e369d15eda08694],
        [0x3f2e3c60324a40e6, 0x3fe359400999418d, 0x3fbb9028b07eb09b, 0x3efa0b274d8cf970, 0x3e389b980cc956da],
        [0x3f2f627cdf3411da, 0x3fe3645c31490c69, 0x3fbbd689dbcb244c, 0x3efb4d8334a079d7, 0x3e3ac71f0799a4f2],
        [0x3f3049e339e8aeaf, 0x3fe36f7eba15ccdc, 0x3fbc1d9ebc10073d, 0x3efc9f752a488273, 0x3e3d23a32ec00734],
        [0x3f30e854d274a2e0, 0x3fe37aa7a7a93e96, 0x3fbc65691c2a439e, 0x3efe01be1a2c802d, 0x3e3fb576727fac08],
        [0x3f318ccba456647b, 0x3fe385d6fdaf37b8, 0x3fbcadeacb8a6c4c, 0x3eff752843dc8640, 0x3e4140a624fad69b],
        [0x3f3237823f723f01, 0x3fe3910cbfd5aa0e, 0x3fbcf7259e406c85, 0x3f007d43d7230a38, 0x3e42c62126652080],
        [0x3f32e8b56d52933c, 0x3fe39c48f1cca43e, 0x3fbd411b6d075578, 0x3f01495d505ef8cc, 0x3e446df4c784d8f4],
        [0x3f33a0a446ccf6ea, 0x3fe3a78b97465303, 0x3fbd8bce155149f2, 0x3f021f5510f219a5, 0x3e463b2868221c78],
        [0x3f345f904a79e05c, 0x3fe3b2d4b3f70263, 0x3fbdd73f7953887f, 0x3f02ffa53e55712d, 0x3e483107c84b2dc4],
        [0x3f3525bd7406de1d, 0x3fe3be244b951ee6, 0x3fbe237180129448, 0x3f03eacde5e5967f, 0x3e4a53290ff00569],
        [0x3f35f372546ba877, 0x3fe3c97a61d936d2, 0x3fbe7066156e7cfe, 0x3f04e15545fc6172, 0x3e4ca5735e9d3a07],
        [0x3f36c8f82b0aab7c, 0x3fe3d4d6fa7dfb60, 0x3fbebe1f2a2f461c, 0x3f05e3c81a9368a2, 0x3e4f2c25ef574127],
        [0x3f37a69affc5fc16, 0x3fe3e03a194041f9, 0x3fbf0c9eb4116de1, 0x3f06f2b9ed9b0eec, 0x3e50f5efeed3033e],
        [0x3f388ca9be120190, 0x3fe3eba3c1df056f, 0x3fbf5be6add29432, 0x3f080ec56b33fd50, 0x3e5274d44d03dd43],
        [0x3f397b76510f7795, 0x3fe3f713f81b673b, 0x3fbfabf9173e41e3, 0x3f09388cb9fb1df7, 0x3e54157c8ea14f6a],
        [0x3f3a7355c0b6ca18, 0x3fe4028abfb8b0b7, 0x3fbffcd7f53ad08c, 0x3f0a70b9d79a8003, 0x3e55dae2f5b5631b],
        [0x3f3b74a0501f2e4d, 0x3fe40e081c7c545b, 0x3fc02742a8eb39af, 0x3f0bb7fef9d3fc05, 0x3e57c844fc765e05],
        [0x3f3c7fb19cec4082, 0x3fe4198c122deefa, 0x3fc050819e2a309c, 0x3f0d0f16f43cfeab, 0x3e59e12942c5c19e],
        [0x3f3d94e8bfed57e8, 0x3fe42516a4974903, 0x3fc07a29e49d1035, 0x3f0e76c5a2e58d47, 0x3e5c2966018129b2],
        [0x3f3eb4a86efa2dd3, 0x3fe430a7d78457bf, 0x3fc0a43c892e7a26, 0x3f0fefd85a276657, 0x3e5ea5280f720e1c],
        [0x3f3fdf572018e74c, 0x3fe43c3faec33e8d, 0x3fc0ceba9b77b6a0, 0x3f10bd932dee88fd, 0x3e60ac7d4259ea3d],
        [0x3f408aaf96fd0277, 0x3fe447de2e245029, 0x3fc0f9a52dc78da2, 0x3f118cc8a921e62d, 0x3e6224e78544c27c],
        [0x3f412b977eeb9b64, 0x3fe45383597a0fe8, 0x3fc124fd552931bf, 0x3f126602e867380f, 0x3e63be83736bf624],
        [0x3f41d29c9361482e, 0x3fe45f2f349932fb, 0x3fc150c4296b3c8e, 0x3f1349bded9975a6, 0x3e657c3e69e81ecb],
        [0x3f427ffa4d3a0e0d, 0x3fe46ae1c358a1b5, 0x3fc17cfac526bcf3, 0x3f14387bb97fabc8, 0x3e676147dae66afe],
        [0x3f4333ee67d22ad7, 0x3fe4769b099178c8, 0x3fc1a9a245c6576b, 0x3f1532c49603c44a, 0x3e697117217c212d],
        [0x3f43eeb8f7014ffa, 0x3fe4825b0b1f0a90, 0x3fc1d6bbcb8d7887, 0x3f16392763ffe60a, 0x3e6baf71d9088068],
        [0x3f44b09c7debaf09, 0x3fe48e21cbdee052, 0x3fc20448799f99ce, 0x3f174c39ecced7e9, 0x3e6e2072c3bee906],
        [0x3f4579de06b0f7ae, 0x3fe499ef4fb0bb86, 0x3fc2324976079925, 0x3f186c9937cdf683, 0x3e706448a67c8ec7],
        [0x3f464ac53b01b5f2, 0x3fe4a5c39a769719, 0x3fc260bfe9bf2303, 0x3f199ae9e4018cde, 0x3e71d654da8735c2],
        [0x3f47239c7da3d1c5, 0x3fe4b19eb014a8ba, 0x3fc28fad00b62f87, 0x3f1ad7d8860ebc6c, 0x3e736902fad1564d],
        [0x3f4804b104ef45dc, 0x3fe4bd8094716219, 0x3fc2bf11e9da92b4, 0x3f1c241a0ac09a25, 0x3e751f33b758c57c],
        [0x3f48ee52f64c7c59, 0x3fe4c9694b757238, 0x3fc2eeefd71f9ffd, 0x3f1d806c1e50be8e, 0x3e76fc08b704d7f0],
        [0x3f49e0d582be1b72, 0x3fe4d558d90bc6af, 0x3fc31f47fd85e14f, 0x3f1eed9598ae3e94, 0x3e7902ea523d91cd],
        [0x3f4adc8f04806b8f, 0x3fe4e14f41218cf9, 0x3fc3501b9522e1d1, 0x3f20363377806e89, 0x3e7b378dced78110],
        [0x3f4be1d91dc8e314, 0x3fe4ed4c87a633bd, 0x3fc3816bd9290c88, 0x3f20fedd5554a0da, 0x3e7d9dfc28bbef00],
        [0x3f4cf110d8b0cad8, 0x3fe4f950b08b6c17, 0x3fc3b33a07ef9f13, 0x3f21d13af3011223, 0x3e801d4cb9db470e],
        [0x3f4e0a96c856587d, 0x3fe5055bbfc52aeb, 0x3fc3e58762fab0b4, 0x3f22adc46794d202, 0x3e81891671f4fb1b],
        [0x3f4f2ecf2b400adb, 0x3fe5116db949aa2a, 0x3fc418552f034dd6, 0x3f2394f7988eba37, 0x3e8314f4c5ce91b5],
        [0x3f502f1107874371, 0x3fe51d86a1116a25, 0x3fc44ba4b3ffa84a, 0x3f24875881bc1261, 0x3e84c3bbef2ff7ea],
        [0x3f50cc7dbac4d580, 0x3fe529a67b1732da, 0x3fc47f773d2b5c69, 0x3f2585718090c846, 0x3e8698800576c3fd],
        [0x3f516fe5bd8b0525, 0x3fe535cd4b581543, 0x3fc4b3ce190fcb53, 0x3f268fd3a3243f8c, 0x3e88969a9f5e5aab],
        [0x3f5219833f50409e, 0x3fe541fb15d36ca5, 0x3fc4e8aa998c8a89, 0x3f27a716faffd024, 0x3e8ac1b0f3ecae61],
        [0x3f52c992a5870d90, 0x3fe54e2fde8adfe1, 0x3fc51e0e13dfe909, 0x3f28cbdaf3ee390f, 0x3e8d1dba85b9e1bb],
        [0x3f538052a11f848d, 0x3fe55a6ba98262c6, 0x3fc553f9e0af8a2f, 0x3f29fec6aefd95b7, 0x3e8faf0864c63387],
        [0x3f543e0444d9fdf5, 0x3fe566ae7ac03766, 0x3fc58a6f5c111690, 0x3f2b408961e7ca25, 0x3e913d268a128586],
        [0x3f5502eb1c72e2f1, 0x3fe572f8564cef61, 0x3fc5c16fe5930304, 0x3f2c91dabb17dcd8, 0x3e92c25290f7d9ff],
        [0x3f55cf4d44afe2cd, 0x3fe57f4940336d41, 0x3fc5f8fce0456e18, 0x3f2df37b4a85578b, 0x3e9469d0409c4123],
        [0x3f56a37384571d18, 0x3fe58ba13c80e5cb, 0x3fc63117b2c31427, 0x3f2f6634efa18aba, 0x3e9636a65b8d0030],
        [0x3f577fa96619244e, 0x3fe598004f44e152, 0x3fc669c1c73a5a4c, 0x3f30756da64aba61, 0x3e982c1ff6c00160],
        [0x3f58643d5377134e, 0x3fe5a4667c913d11, 0x3fc6a2fc8b76706e, 0x3f3141261f890b19, 0x3e9a4dd27ff66085],
        [0x3f595180b0ae4985, 0x3fe5b0d3c87a2c80, 0x3fc6dcc970e88a9c, 0x3f3216b82f7c1586, 0x3e9c9fa44c22b5eb],
        [0x3f5a47c7f9b3bf0c, 0x3fe5bd4837163aaa, 0x3fc71729ecb131f6, 0x3f32f69dc18fcb54, 0x3e9f25d3b9d1b072],
        [0x3f5b476ae049427e, 0x3fe5c9c3cc7e4b8a, 0x3fc7521f77a9ad5f, 0x3f33e156a6452735, 0x3ea0f27f7a512b32],
        [0x3f5c50c46b3756db, 0x3fe5d6468ccd9d5c, 0x3fc78dab8e6d823a, 0x3f34d768dc291d28, 0x3ea271163400f636],
        [0x3f5d643316b6cfd9, 0x3fe5e2d07c21ca00, 0x3fc7c9cfb1640d67, 0x3f35d960dc52aca1, 0x3ea41169f85012ca],
        [0x3f5e8218f615b73a, 0x3fe5ef619e9ac84d, 0x3fc8068d64ca34c1, 0x3f36e7d1ea93bf57, 0x3ea5d6746eb6948e],
        [0x3f5faadbd6a3761b, 0x3fe5fbf9f85aed73, 0x3fc843e630bc315c, 0x3f380356698a9bf1, 0x3ea7c3726937553d],
        [0x3f606f72b1f858be, 0x3fe608998d86ee50, 0x3fc881dba13f72c8, 0x3f392c9032c3ea3a, 0x3ea9dbe9d0ab3ca3],
        [0x3f610f51a6b7e144, 0x3fe615406245e0d4, 0x3fc8c06f464c9b8d, 0x3f3a6428f31f98e8, 0x3eac23b016c344e6],
        [0x3f61b543b6c1982c, 0x3fe621ee7ac13d59, 0x3fc8ffa2b3d99726, 0x3f3baad28bad634f, 0x3eae9ef1398bda81],
        [0x3f626183f90409cb, 0x3fe62ea3db24e006, 0x3fc93f7781e3c9c6, 0x3f3d014777384297, 0x3eb0a91bb2a3352e],
        [0x3f63144fc33569bb, 0x3fe63b60879f0a29, 0x3fc97fef4c7a5a15, 0x3f3e684b34bac245, 0x3eb22139a147cdbd],
        [0x3f63cde6bfaa9bee, 0x3fe648248460639f, 0x3fc9c10bb3c89537, 0x3f3fe0aab6f8fd59, 0x3eb3ba828001f476],
        [0x3f648e8b0402af47, 0x3fe654efd59bfc2c, 0x3fca02ce5c206d5b, 0x3f40b59e6cbffcdd, 0x3eb577e313f5aa18],
        [0x3f65568128aedc26, 0x3fe661c27f874ce2, 0x3fca4538ee051316, 0x3f4184716da695f8, 0x3eb75c8a29f51535],
        [0x3f662610615f67ed, 0x3fe66e9c865a3984, 0x3fca884d1635a9cb, 0x3f425d446fb2140b, 0x3eb96bee69258225],
        [0x3f66fd82965e200b, 0x3fe67b7dee4f11e6, 0x3fcacc0c85b8176f, 0x3f43409339cdf3be, 0x3ebba9d4a918139a],
        [0x3f67dd247edf739b, 0x3fe68866bba29353, 0x3fcb1078f1e3efe1, 0x3f442edf8ef84b84, 0x3ebe1a56d6edb376],
        [0x3f68c545bc538a98, 0x3fe69556f293e9f0, 0x3fcb5594146d7c27, 0x3f4528b178554d81, 0x3ec060f5bb11c291],
        [0x3f69b638f6c114b0, 0x3fe6a24e9764b225, 0x3fcb9b5fab70ddd7, 0x3f462e9792d7adab, 0x3ec1d2b6e560efe5],
        [0x3f6ab053fa33e870, 0x3fe6af4dae58fa01, 0x3fcbe1dd797d4ee7, 0x3f47412760aa4130, 0x3ec365135deedc7b],
        [0x3f6bb3efd549eda9, 0x3fe6bc543bb742a2, 0x3fcc290f45a07e45, 0x3f4860fd9e8950ca, 0x3ec51aeb3f56ced1],
        [0x3f6cc168f8e933fb, 0x3fe6c96243c8819c, 0x3fcc70f6db720960, 0x3f498ebe9d3c570d, 0x3ec6f75f8df1253f],
        [0x3f6dd91f592a8024, 0x3fe6d677cad82262, 0x3fccb9960b1f1311, 0x3f4acb169f633eb1, 0x3ec8fdd7f14120fd],
        [0x3f6efb768f840894, 0x3fe6e394d53407b3, 0x3fcd02eea975f809, 0x3f4c16ba3bccad22, 0x3ecb3208ee9d21c4],
        [0x3f70146aff2045ed, 0x3fe6f0b9672c8cff, 0x3fcd4d028ff22132, 0x3f4d7266c48d7c69, 0x3ecd97fab076aec9],
        [0x3f70b0d47aa7b0e0, 0x3fe6fde5851487d8, 0x3fcd97d39cc7f434, 0x3f4edee2b3143e6c, 0x3ed01a083354770f],
        [0x3f71532f6c3d515c, 0x3fe70b1933414958, 0x3fcde363b2f0e27d, 0x3f502e7f0cbb3ddf, 0x3ed1858826a60f97],
        [0x3f71fbb5a388807d, 0x3fe71854760a9f94, 0x3fce2fb4ba379711, 0x3f50f6c98c9c2e1e, 0x3ed31116345ea8ef],
        [0x3f72aaa32288b953, 0x3fe7259751cad705, 0x3fce7cc89f444372, 0x3f51c8c32fe943e9, 0x3ed4bf86036a8a2e],
        [0x3f73603632f3ab1a, 0x3fe732e1cadebbf9, 0x3fcecaa153a90bf5, 0x3f52a4e3d49eaada, 0x3ed693eb0b584819],
        [0x3f741caf7c63242b, 0x3fe74033e5a59c03, 0x3fcf1940cdee93cb, 0x3f538ba92465e7b6, 0x3ed8919e34fc8d42],
        [0x3f74e0521b5abb93, 0x3fe74d8da681476a, 0x3fcf68a909a0a913, 0x3f547d96dc52540f, 0x3edabc43fa20b40d],
        [0x3f75ab63b92d6bde, 0x3fe75aef11d61297, 0x3fcfb8dc075b114c, 0x3f557b37181587ae, 0x3edd17d30f703597],
        [0x3f767e2ca4cba173, 0x3fe768582c0ad78e, 0x3fd004ede66b3b39, 0x3f56851aa0d69e31, 0x3edfa89ba4d5dadf],
        [0x3f7758f7ec829194, 0x3fe775c8f988f75b, 0x3fd02dd5327aba8f, 0x3f579bd93fd95ab8, 0x3ee139a7a4c66a84],
        [0x3f783c1378b60516, 0x3fe783417ebc5b88, 0x3fd05724efe8e5fc, 0x3f58c012152458ba, 0x3ee2be84c12aaf64],
        [0x3f7927d0279e1b03, 0x3fe790c1c013778e, 0x3fd080de2964d66b, 0x3f59f26bf257c0e4, 0x3ee465ac90c1374b],
        [0x3f7a1c81ea12e3ee, 0x3fe79e49c1ff4a50, 0x3fd0ab01ec4697f3, 0x3f5b3395b9e85d61, 0x3ee6322538fc5245],
        [0x3f7b1a7fe17016ce, 0x3fe7abd988f35f8d, 0x3fd0d5914895f48a, 0x3f5c8446c2f56a81, 0x3ee8273923dc43d7],
        [0x3f7c22247e9b8417, 0x3fe7b9711965d156, 0x3fd1008d51115024, 0x3f5de53f41f22205, 0x3eea487d05190c1f],
        [0x3f7d33cda239530c, 0x3fe7c71077cf4989, 0x3fd12bf71b349642, 0x3f5f5748b65ec03d, 0x3eec99d667345a74],
        [0x3f7e4fdcbe1980fb, 0x3fe7d4b7a8ab0349, 0x3fd157cfbf40393d, 0x3f606d9b2ee7d311, 0x3eef1f82cc6fcbd3],
        [0x3f7f76b6f7ea898a, 0x3fe7e266b076cc73, 0x3fd184185840436c, 0x3f6138f2d642215b, 0x3ef0ef0fb859139f],
        [0x3f805462a69ec975, 0x3fe7f01d93b30720, 0x3fd1b0d204137a59, 0x3f620e1f65e94d56, 0x3ef26d58dd465aee],
        [0x3f80f33a5c747891, 0x3fe7fddc56e2ab17, 0x3fd1ddfde3729428, 0x3f62ed9a8f57ccc1, 0x3ef40d5835691bd0],
        [0x3f81981b2ceb281a, 0x3fe80ba2fe8b4751, 0x3fd20b9d19f77f6c, 0x3f63d7e3e64fea44, 0x3ef5d206cdc6cc1a],
        [0x3f82433fcda52b46, 0x3fe819718f350372, 0x3fd239b0ce24bd92, 0x3f64cd8129b0085a, 0x3ef7bea0d050f51a],
        [0x3f82f4e52f5a0bb2, 0x3fe827480d6aa146, 0x3fd2683a296cd00f, 0x3f65cefe8fcc58f1, 0x3ef9d6ab6efd0287],
        [0x3f83ad4a93899c36, 0x3fe835267db97e42, 0x3fd2973a5839b88f, 0x3f66dcef16799fbf, 0x3efc1dfb5477b04a],
        [0x3f846cb1a3021f5d, 0x3fe8430ce4b19500, 0x3fd2c6b289f48c38, 0x3f67f7ecd6f6b09c, 0x3efe98bba63c52bc],
        [0x3f85335e854186c9, 0x3fe850fb46e57ec3, 0x3fd2f6a3f10d1a5d, 0x3f6920995de49109, 0x3f00a5bad27118c0],
        [0x3f860197f8ba1e98, 0x3fe85ef1a8ea74f7, 0x3fd3270fc301a6a3, 0x3f6a579e077f749f, 0x3f021d8c7c49ceba],
        [0x3f86d7a76c03491c, 0x3fe86cf00f5852af, 0x3fd357f73866b6ef, 0x3f6b9dac604d3a11, 0x3f03b6825c6eb102],
        [0x3f87b5d917ff431b, 0x3fe87af67ec9962f, 0x3fd3895b8ceef543, 0x3f6cf37e8a7899dd, 0x3f057388a02df7c1],
        [0x3f889c7c1aff400e, 0x3fe88904fbdb6269, 0x3fd3bb3dff7325ac, 0x3f6e59d7a812e2d8, 0x3f0757cd6f204965],
        [0x3f898be294ef8885, 0x3fe8971b8b2d8085, 0x3fd3ed9fd1fa309b, 0x3f6fd1844a78ed49, 0x3f0966c6bc9f7509],
        [0x3f8a8461c495a2ad, 0x3fe8a53a31626166, 0x3fd4208249c141ab, 0x3f70adad738d6dcf, 0x3f0ba4389c96357e],
        [0x3f8b865225eaeeb9, 0x3fe8b360f31f1f31, 0x3fd453e6af43fb3c, 0x3f717c1e28f4a9ac, 0x3f0e143c273e331d],
        [0x3f8c920f919e8606, 0x3fe8c18fd50b7ed1, 0x3fd487ce4e44beef, 0x3f72548a1ed708c2, 0x3f105da37c337b42],
        [0x3f8da7f95dc99524, 0x3fe8cfc6dbd1f184, 0x3fd4bc3a75d50b4b, 0x3f73376ce14c8fca, 0x3f11cf19abfe851c],
        [0x3f8ec8727fe1d5ef, 0x3fe8de060c1f965e, 0x3fd4f12c785deec7, 0x3f742547f5a7bbfe, 0x3f1361248d5f1622],
        [0x3f8ff3e1aff6403a, 0x3fe8ec4d6aa43bd9, 0x3fd526a5aba8905d, 0x3f751ea32465d06c, 0x3f1516a3a5ac3113],
        [0x3f909558c6a13e87, 0x3fe8fa9cfc12615c, 0x3fd55ca768e6cde9, 0x3f76240cc6b253be, 0x3f16f2b756d034a1],
        [0x3f9136a862131160, 0x3fe908f4c51f38c5, 0x3fd593330cbbf091, 0x3f77361a17a9fe83, 0x3f18f8c6978d7de4],
        [0x3f91de191ac69f41, 0x3fe91754ca82a7fb, 0x3fd5ca49f745775d, 0x3f785567898b7d6f, 0x3f1b2c852ce346f1],
        [0x3f928be68fec19e9, 0x3fe925bd10f74a77, 0x3fd601ed8c23f848, 0x3f7982991f06a97b, 0x3f1d91fa6ff5ceb2],
        [0x3f93404ea4a8c155, 0x3fe9342d9d3a72d2, 0x3fd63a1f328417ff, 0x3f7abe5ac8dd31c3, 0x3f2016c4566ef3c0],
    ];

    /// The same for deflections inside a swing, smaller than any amplitude of
    /// the card's range: 40 values `1e-12 · (5e-5 / 1e-12)^(i / 40)` for
    /// `i = 0..39`.
    const POWER_INSIDE_A_SWING: [[u64; 5]; 40] = [
        [0x3d719799812dea11, 0x3fc85686bce44950, 0x3f44fbecac11f254, 0x3cc7127e52dd83f4, 0x3a495e421b58af2d],
        [0x3d7b671a414122fe, 0x3fc8fea52c034884, 0x3f479c65f4e2a324, 0x3cd43808ce0408b5, 0x3a61506c60eca589],
        [0x3d8557979cacdc99, 0x3fc9ab4cea885ef1, 0x3f4a910de90252bc, 0x3ce1b7e95d474ebe, 0x3a77a2623483bc0b],
        [0x3d909f399c98e4dd, 0x3fca5c9d4e66bd70, 0x3f4de46e4f9b6ece, 0x3cef0de7fa2c5c39, 0x3a902178fe769b31],
        [0x3d99e437ceb09ac1, 0x3fcb12b686069543, 0x3f50d13153928a25, 0x3cfb36bcb9d46a2d, 0x3aa604d8e7b4eb62],
        [0x3da42a4619fdb6b8, 0x3fcbcdb99e1c4ee9, 0x3f52ec21360fbed2, 0x3d07d928428fe62a, 0x3abe0e74d9b035b3],
        [0x3daf6918c5601d59, 0x3fcc8dc887a81973, 0x3f554a8855518be7, 0x3d14e6211d37fd85, 0x3ad483935e7674a4],
        [0x3db876ab92ff0fe1, 0x3fcd53061e1ee8f3, 0x3f57f4d8c22e06b3, 0x3d225079e360ec5e, 0x3aec008cbc574876],
        [0x3dc30d92be313eba, 0x3fce1d962dbe024c, 0x3f5af49334e3a231, 0x3d300ca65f49eb78, 0x3b031ca30a8af685],
        [0x3dcdada0dd6ea1c3, 0x3fceed9d7a0a3a4b, 0x3f5e5468eeb4ad2c, 0x3d3c210fe78ab76a, 0x3b1a16968fc6978d],
        [0x3dd71d48501dfc02, 0x3fcfc341c47c15ed, 0x3f611030eca65356, 0x3d48a680cb291ce8, 0x3b31ce3b2b16ce27],
        [0x3de20092eb3441f2, 0x3fd04f54e9ad00e6, 0x3f633303b5edc194, 0x3d559a147869514e, 0x3b484e1d62fee0ad],
        [0x3dec0a9e0cbb61ca, 0x3fd0bffebc60ef8f, 0x3f659a4a776b6f6b, 0x3d62ee2c122b5c47, 0x3b6096ae7d2d6233],
        [0x3df5d6f16513133f, 0x3fd133b2ccf1969d, 0x3f684e96e53b21be, 0x3d7096d7f432c187, 0x3b76a4d741c58d65],
        [0x3e010268f3639455, 0x3fd1aa861b358fb2, 0x3f6b598d50c52e60, 0x3d7d1344bd9c70d0, 0x3b8ee8da299654a9],
        [0x3e0a7eb7065004c2, 0x3fd2248e3811fbeb, 0x3f6ec6070946022d, 0x3d897ac175c7b76c, 0x3ba518a2447b1bec],
        [0x3e14a299e36956bf, 0x3fd2a1e149648508, 0x3f71501c85017856, 0x3d9654154b27cd64, 0x3bbccc04b6f2415d],
        [0x3e20124362dd8bb3, 0x3fd322960e086812, 0x3f737aefc00aca72, 0x3da3912c18ed8361, 0x3bd3a781d35a5519],
        [0x3e2908a58853346e, 0x3fd3a6c3e1f7445c, 0x3f75eb37617d5525, 0x3db125af743881e8, 0x3bead426640d44cc],
        [0x3e337f43b191c161, 0x3fd42e82c2866e86, 0x3f78a9a5373f133b, 0x3dbe0d9f191aa207, 0x3c024f9c199cc9d0],
        [0x3e3e5eb8a5cd53ec, 0x3fd4b9eb52c18c9f, 0x3f7bc001b13ce235, 0x3dca5625baf38993, 0x3c18feb865e79ed6],
        [0x3e47a7354f2b84ec, 0x3fd54916dfe3459b, 0x3f7f394ec2b985eb, 0x3dd71457b327f41d, 0x3c310f37a53e8c67],
        [0x3e526bfeb97cebfa, 0x3fd5dc1f65ecd3e8, 0x3f8190f790b4771e, 0x3de439a7a3622b1b, 0x3c47496026a0de24],
        [0x3e5cb1f18da67164, 0x3fd6731f945d507e, 0x3f83c3e93722d359, 0x3df1b954e529e2bb, 0x3c5fc97260a8c47b],
        [0x3e66594317764d67, 0x3fd70e32d3099183, 0x3f863d5372c951e2, 0x3dff10651f6d58ec, 0x3c75b1ec3ff4ef55],
        [0x3e7167e822c11884, 0x3fd7ad7547157d87, 0x3f890608a398a3e7, 0x3e0b38eb13476a65, 0x3c8d9d43223a1026],
        [0x3e7b1cd023dedff6, 0x3fd85103d80fba7f, 0x3f8c27f5df506170, 0x3e17db118edd334b, 0x3ca43651aa978766],
        [0x3e851dbbae0118ec, 0x3fd8f8fc3530a5ca, 0x3f8fae465577f58b, 0x3e24e7cde683e5de, 0x3cbb97179c4fb122],
        [0x3e90722997a9992c, 0x3fd9a57cdabd88fa, 0x3f91d2c590bf9465, 0x3e3251f1a57341bf, 0x3cd2d4a91ed0d85f],
        [0x3e999e068bce57bb, 0x3fda56a5179105c2, 0x3f940df40c8030fb, 0x3e400defa8fd8e73, 0x3ce9b456a832f352],
        [0x3ea3f39b0cffe1ae, 0x3fdb0c9512c9ba07, 0x3f9690a31af24f33, 0x3e4c235108aa6cee, 0x3d018b2ca2fdbf14],
        [0x3eaf13f1292b08eb, 0x3fdbc76dd1a02425, 0x3f9963c628145043, 0x3e58a87a8c91d2b2, 0x3d17f2949584b3a4],
        [0x3eb83459526a6b3d, 0x3fdc87513d64d685, 0x3f9c916f78c1971b, 0x3e659bcfadc722e8, 0x3d30583544d079d0],
        [0x3ec2d9eb85c53327, 0x3fdd4c6229a810f3, 0x3fa0127a099f6b8e, 0x3e72efb077b5f5c8, 0x3d464f900ec5f532],
        [0x3ecd5d2b82285009, 0x3fde16c45a8bddb4, 0x3fa2158a134351de, 0x3e80982c513cd0d0, 0x3d5e7471f4cbbdcf],
        [0x3ed6de9e6a51849a, 0x3fdee69c8b41d7f2, 0x3fa4591440321a0b, 0x3e8d1599481d66aa, 0x3d74c92f343ef98e],
        [0x3ee1cfc4f64b9c2f, 0x3fdfbc1074b5c938, 0x3fa6e52ada396609, 0x3e997ccc3a00d8d2, 0x3d8c5f9150ec4da4],
        [0x3eebbe98a46b64ff, 0x3fe04ba36a3329d3, 0x3fa9c2e2d5315177, 0x3ea655df68c625b3, 0x3da35d7ce98d8897],
        [0x3ef59bbc362c150c, 0x3fe0bc33b9b67379, 0x3facfc74305c62c3, 0x3eb392bd8ec36eaf, 0x3dba6f1c96300108],
        [0x3f00d44c09e998b1, 0x3fe12fcd96db2381, 0x3fb04eaf32bbf015, 0x3ec1270f43f6b343, 0x3dd20aa652641ff3],
    ];

    fn blank_frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: -1.0,
            pos: [1.0, 2.0, 3.0],
            vel: [4.0, 5.0, 6.0],
            force_torque: 7.0,
            residual: 8.0,
            flags: 9,
            proof: [0xA5; 16],
        }
    }

    fn proof_intact(frame: &LastStateFrame64) {
        let mut index = 0;
        while index < 16 {
            assert_eq!(frame.proof[index], 0xA5, "proof byte {index} was resealed");
            index += 1;
        }
    }

    fn assert_copy<T: Copy>() {}

    fn state_bits(state: &CantileverState) -> [u64; 11] {
        [
            state.t_s.to_bits(),
            state.deflection_m.to_bits(),
            state.velocity_m_s.to_bits(),
            state.envelope_m.to_bits(),
            state.kinetic_j_per_kg.to_bits(),
            state.potential_j_per_kg.to_bits(),
            state.stored_j_per_kg.to_bits(),
            state.energy_anchor_j_per_kg.to_bits(),
            state.heat_j_per_kg.to_bits(),
            state.energy_defect_j_per_kg.to_bits(),
            state.residual_conservation_error.to_bits(),
        ]
    }

    fn relative(got: f64, expected: f64) -> f64 {
        ((got - expected) / expected).abs()
    }

    fn within(got: f64, expected: f64, tolerance: f64) -> bool {
        (got - expected).abs() <= tolerance * expected.abs()
    }

    /// The beam and the start of the scored ring-down on the compared run.
    fn scored_start() -> (CantileverEth, CantileverState) {
        let beam = CantileverEth::first_bending_mode();
        let state = CantileverState::try_phase(
            &beam,
            SCORED_START_DEFLECTION_M,
            SCORED_START_VELOCITY_M_S,
        )
        .unwrap();
        (beam, state)
    }

    /// `steps` steps of the Runge–Kutta map of `integrator` from `y`, with no
    /// bound applied, booked by the ledger of `ledger` exactly as `commit`
    /// books a step: the largest residual, and where the map ends.
    fn ledger_on_map(
        ledger: &CantileverEth,
        integrator: &CantileverEth,
        start: [f64; 3],
        dt_s: f64,
        steps: u64,
    ) -> (f64, [f64; 3]) {
        let anchor = ledger.stored_j_per_kg(start[0], start[1]);
        let scale = ledger.ledger_scale_j_per_kg();
        let mut y = start;
        let mut worst = 0.0_f64;
        let mut count = 0;
        while count < steps {
            y = rk4(integrator, y, dt_s).unwrap();
            let defect = ledger.stored_j_per_kg(y[0], y[1]) - anchor + y[2];
            let residual = defect.abs() / scale;
            if residual > worst {
                worst = residual;
            }
            count += 1;
        }
        (worst, y)
    }

    /// The real `validate`, a chosen integrator, the real `commit`.
    fn step_with(
        beam: &CantileverEth,
        state: &mut CantileverState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        integrate: impl FnOnce(&CantileverEth, [f64; 3], f64) -> Option<[f64; 3]>,
    ) -> Result<(), StepReject> {
        validate(beam, state, NO_LOAD, dt_s)?;
        let advanced = match integrate(beam, state_vector(state), dt_s) {
            Some(y) => y,
            None => return Err(StepReject::NonFiniteState),
        };
        commit(beam, state, frame, dt_s, advanced)
    }

    /// The real `validate` and `commit`, the real Runge–Kutta step, one
    /// alteration of its result.
    fn step_altered(
        beam: &CantileverEth,
        state: &mut CantileverState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        alter: impl FnOnce(&CantileverState, &mut [f64; 3]),
    ) -> Result<(), StepReject> {
        let held = *state;
        step_with(beam, state, frame, dt_s, |beam, y, dt_s| {
            let mut out = rk4(beam, y, dt_s)?;
            alter(&held, &mut out);
            Some(out)
        })
    }

    /// Explicit Euler on deflection, speed and heat.
    fn euler(beam: &CantileverEth, y: [f64; 3], dt_s: f64) -> Option<[f64; 3]> {
        Some(advance(y, dt_s, rates(beam, y)?))
    }

    /// Midpoint rule, second order.
    fn midpoint(beam: &CantileverEth, y: [f64; 3], dt_s: f64) -> Option<[f64; 3]> {
        let k1 = rates(beam, y)?;
        let k2 = rates(beam, advance(y, 0.5 * dt_s, k1))?;
        Some(advance(y, dt_s, k2))
    }

    /// What a ring-down shows, period by period, as
    /// `analysis/05_card_emulation.py` reads it: the times of the upward zero
    /// crossings (straight line between two steps) and the largest deflection
    /// of the period that follows each.
    struct Reading {
        crossing_s: Vec<f64>,
        amplitude_m: Vec<f64>,
    }

    impl Reading {
        fn frequency_hz(&self, period: usize) -> f64 {
            1.0 / (self.crossing_s[period + 1] - self.crossing_s[period])
        }

        /// Decay rate of the amplitude over the eight periods from `period`.
        fn decay_rate_per_s(&self, period: usize) -> f64 {
            crate::math::log(self.amplitude_m[period] / self.amplitude_m[period + 8])
                / (self.crossing_s[period + 8] - self.crossing_s[period])
        }
    }

    /// 24 s of the Runge–Kutta map at the ceiling from the start of the
    /// compared run, with the constants of `beam`, which may be altered.
    fn read_ring_down(beam: &CantileverEth) -> Reading {
        let mut y = [START_DEFLECTION_M, START_VELOCITY_M_S, 0.0];
        let mut crossing_s = Vec::new();
        let mut amplitude_m = Vec::new();
        let mut largest_m = 0.0_f64;
        let mut t_s = 0.0;
        let mut count = 0;
        while count < 96_000 {
            let next = rk4(beam, y, GATE_DT_S).unwrap();
            if next[0].abs() > largest_m {
                largest_m = next[0].abs();
            }
            if y[0] < 0.0 && next[0] >= 0.0 {
                crossing_s.push(t_s + GATE_DT_S * (-y[0]) / (next[0] - y[0]));
                amplitude_m.push(largest_m);
                largest_m = 0.0;
            }
            y = next;
            t_s += GATE_DT_S;
            count += 1;
        }
        // The first entry is the part of a period before the first crossing.
        amplitude_m.remove(0);
        Reading {
            crossing_s,
            amplitude_m,
        }
    }

    // Spec test 1.
    #[test]
    fn the_card_locks_its_constants() {
        assert_eq!(SMALL_AMPLITUDE_FREQUENCY_HZ.to_bits(), 8.29235_f64.to_bits());
        assert_eq!(SPRING_KAPPA.to_bits(), (-0.183144_f64).to_bits());
        assert_eq!(SPRING_POWER.to_bits(), 1.26613_f64.to_bits());
        assert_eq!(DAMPING_C.to_bits(), 0.673442_f64.to_bits());
        assert_eq!(DAMPING_POWER.to_bits(), 0.0600771_f64.to_bits());
        assert_eq!(DRAG_PER_M.to_bits(), 0.793247_f64.to_bits());
        assert_eq!(AMPLITUDE_CEILING_M.to_bits(), 0.01875_f64.to_bits());
        assert_eq!(COMPARED_CEILING_M.to_bits(), 0.01579_f64.to_bits());
        // 3.119 V, the largest amplitude of the fitted recording, times the
        // metres per volt, rounded down to the ceiling.
        assert!(3.119 * 0.006012 > AMPLITUDE_CEILING_M && 3.119 * 0.006012 < AMPLITUDE_CEILING_M + 2.0e-6);
        assert!(AMPLITUDE_CEILING_M < LEDGER_SCALE_AMPLITUDE_M);
        assert_eq!(FLAG_ENVELOPE_BELOW_COMPARED, 1);
        assert_eq!(FLAG_ENVELOPE_ABOVE_COMPARED, 2);
        assert_eq!(AMPLITUDE_FLOOR_M.to_bits(), 5.8e-5_f64.to_bits());
        assert_eq!(LEDGER_SCALE_AMPLITUDE_M.to_bits(), 0.0188_f64.to_bits());
        assert_eq!(GATE_DT_S.to_bits(), 2.5e-4_f64.to_bits());
        assert_eq!(GATE_OVER_FLOOR.to_bits(), 262_144.0_f64.to_bits());
        assert_eq!(RESIDUAL_CONSERVATION_LIMIT.to_bits(), 1.0e-6_f64.to_bits());

        let beam = CantileverEth::first_bending_mode();
        assert!(beam.is_valid());
        assert_eq!(
            beam.small_amplitude_frequency_hz.to_bits(),
            SMALL_AMPLITUDE_FREQUENCY_HZ.to_bits()
        );
        assert_eq!(beam.spring_kappa.to_bits(), SPRING_KAPPA.to_bits());
        assert_eq!(beam.spring_power.to_bits(), SPRING_POWER.to_bits());
        assert_eq!(beam.damping_c.to_bits(), DAMPING_C.to_bits());
        assert_eq!(beam.damping_power.to_bits(), DAMPING_POWER.to_bits());
        assert_eq!(beam.drag_per_m.to_bits(), DRAG_PER_M.to_bits());
        assert_eq!(beam.amplitude_ceiling_m.to_bits(), AMPLITUDE_CEILING_M.to_bits());
        assert_eq!(beam.compared_ceiling_m.to_bits(), COMPARED_CEILING_M.to_bits());
        assert_eq!(beam.amplitude_floor_m.to_bits(), AMPLITUDE_FLOOR_M.to_bits());
        assert_eq!(beam.gate_dt_s.to_bits(), GATE_DT_S.to_bits());
        assert_eq!(beam.gate_over_floor.to_bits(), GATE_OVER_FLOOR.to_bits());
        assert_eq!(
            beam.residual_conservation_limit.to_bits(),
            RESIDUAL_CONSERVATION_LIMIT.to_bits()
        );

        // The derived values of the spec, section 2.
        assert!(within(beam.omega0_rad_s(), 52.10237168199065, 1.0e-15));
        assert!(
            within(beam.ledger_scale_j_per_kg(), 0.4528039711, 1.0e-10),
            "{}",
            beam.ledger_scale_j_per_kg()
        );
        // The spring still restores at the ceiling: 0.9364 of the plain spring.
        assert!(
            (beam.spring_share_at_ceiling() - 0.9364).abs() < 5.0e-4,
            "{}",
            beam.spring_share_at_ceiling()
        );

        // Every recorded copy, changed by one bit, is invalid; a state cannot
        // be made on it and a step on it is refused with nothing changed.
        let mut tampered = [beam; 12];
        tampered[0].small_amplitude_frequency_hz =
            f64::from_bits(beam.small_amplitude_frequency_hz.to_bits() + 1);
        tampered[1].spring_kappa = f64::from_bits(beam.spring_kappa.to_bits() + 1);
        tampered[2].spring_power = f64::from_bits(beam.spring_power.to_bits() + 1);
        tampered[3].damping_c = f64::from_bits(beam.damping_c.to_bits() + 1);
        tampered[4].damping_power = f64::from_bits(beam.damping_power.to_bits() + 1);
        tampered[5].drag_per_m = f64::from_bits(beam.drag_per_m.to_bits() + 1);
        tampered[6].amplitude_ceiling_m = f64::from_bits(beam.amplitude_ceiling_m.to_bits() + 1);
        tampered[7].amplitude_floor_m = f64::from_bits(beam.amplitude_floor_m.to_bits() + 1);
        tampered[8].gate_dt_s = f64::from_bits(beam.gate_dt_s.to_bits() + 1);
        tampered[9].gate_over_floor = f64::from_bits(beam.gate_over_floor.to_bits() + 1);
        tampered[10].residual_conservation_limit =
            f64::from_bits(beam.residual_conservation_limit.to_bits() + 1);
        tampered[11].compared_ceiling_m = f64::from_bits(beam.compared_ceiling_m.to_bits() + 1);
        let (_, fresh) = scored_start();
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        for (index, card) in tampered.iter().enumerate() {
            assert!(!card.is_valid(), "field {index}");
            assert!(
                CantileverState::try_phase(card, SCORED_START_DEFLECTION_M, SCORED_START_VELOCITY_M_S)
                    .is_none(),
                "field {index}"
            );
            let mut state = fresh;
            assert_eq!(
                step(card, &mut state, &mut frame, NO_LOAD, GATE_DT_S),
                Err(StepReject::InvalidBeam),
                "field {index}"
            );
            assert_eq!(state_bits(&state), state_bits(&fresh));
            assert_eq!(frame.to_bytes(), held);
        }

        // The compared ceiling above the ceiling, or under the floor, is invalid.
        let mut wrong = beam;
        wrong.compared_ceiling_m = 0.019;
        assert!(!wrong.is_valid());
        wrong.compared_ceiling_m = 5.0e-5;
        assert!(!wrong.is_valid());

        // A hardening spring, no softening, or a power outside its range is invalid.
        let mut wrong = beam;
        wrong.spring_kappa = 0.183144;
        assert!(!wrong.is_valid());
        wrong.spring_kappa = 0.0;
        assert!(!wrong.is_valid());
        for power in [1.0, 0.9, 3.0, 3.5, f64::NAN] {
            let mut wrong = beam;
            wrong.spring_power = power;
            assert!(!wrong.is_valid(), "spring power {power}");
        }
        for power in [-0.01, 1.0, 1.5, f64::NAN] {
            let mut wrong = beam;
            wrong.damping_power = power;
            assert!(!wrong.is_valid(), "damping power {power}");
        }
        // A spring softened so far that it no longer restores at the ceiling.
        let mut wrong = beam;
        wrong.spring_kappa = -4.0;
        assert!(wrong.spring_share_at_ceiling() < 0.0);
        assert!(!wrong.is_valid());
        let mut state = fresh;
        assert_eq!(
            step(&wrong, &mut state, &mut frame, NO_LOAD, GATE_DT_S),
            Err(StepReject::InvalidBeam)
        );
        assert_eq!(state_bits(&state), state_bits(&fresh));
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);
    }

    #[test]
    fn types_are_copy_and_the_load_is_empty() {
        assert_copy::<CantileverEth>();
        assert_copy::<CantileverState>();
        assert_copy::<CantileverLoad>();
        assert_copy::<StepReject>();
        assert_eq!(core::mem::size_of::<CantileverLoad>(), 0);
        assert_eq!(CantileverLoad::default(), NO_LOAD);
        // The step works on three numbers on the stack.
        assert_eq!(core::mem::size_of::<[f64; 3]>(), 24);
    }

    // Spec test 2, first part. The pass line is a relative 1e-9 on each
    // number: the reference came from Python's power function and this card
    // carries the power as exp(q log x).
    #[test]
    fn the_compared_run_matches_the_reference_after_4000_steps() {
        let beam = CantileverEth::first_bending_mode();
        // The compared run starts at an envelope of 15.786 mm, the largest it
        // held, under the ceiling: the card makes that start.
        let start_envelope_m = beam.envelope_m(START_DEFLECTION_M, START_VELOCITY_M_S);
        assert!((start_envelope_m - 15.786e-3).abs() < 5.0e-8, "{start_envelope_m}");
        assert!(start_envelope_m <= COMPARED_CEILING_M);
        assert!(CantileverState::try_phase(&beam, START_DEFLECTION_M, START_VELOCITY_M_S).is_some());
        // The card's map takes it in 595 steps to the start of the scored
        // ring-down, bit for bit.
        let mut y = [START_DEFLECTION_M, START_VELOCITY_M_S, 0.0];
        let mut count = 0;
        while count < SCORED_START_STEPS {
            y = rk4(&beam, y, GATE_DT_S).unwrap();
            count += 1;
        }
        assert_eq!(y[0].to_bits(), SCORED_START_DEFLECTION_M.to_bits());
        assert_eq!(y[1].to_bits(), SCORED_START_VELOCITY_M_S.to_bits());
        let heat_before_j_per_kg = y[2];

        let (beam, mut state) = scored_start();
        assert!((state.envelope_m - 14.2569e-3).abs() < 5.0e-8, "{}", state.envelope_m);
        assert_eq!(state.t_s, 0.0);
        assert_eq!(state.heat_j_per_kg, 0.0);
        assert_eq!(state.energy_anchor_j_per_kg.to_bits(), state.stored_j_per_kg.to_bits());
        let mut frame = blank_frame();
        let mut count = 0;
        while count < 4_000 - SCORED_START_STEPS {
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            count += 1;
        }
        // The heat restarts at the new anchor; the two parts add up to the
        // heat of the compared run.
        let differences = [
            relative(state.deflection_m, -7.904474048414e-3),
            relative(state.velocity_m_s, 3.130458266039e-1),
            relative(heat_before_j_per_kg + state.heat_j_per_kg, 1.914269575790e-1),
        ];
        println!(
            "after 4,000 steps of the compared run, relative difference from the reference: deflection {:e}, speed {:e}, heat {:e}",
            differences[0], differences[1], differences[2]
        );
        for difference in differences {
            assert!(difference <= 1.0e-9, "{difference:e}");
        }
        assert!((state.t_s - 0.85125).abs() < 1.0e-12);
        // The tight-tolerance reference at the same instant (DOP853, relative
        // tolerance 1e-13, `analysis/out/05_card_emulation.log`): the step at
        // the ceiling is 6.46e-8 m and 1.66e-6 m/s from it.
        let from_reference_m = state.deflection_m - -7.904409444e-3;
        let from_reference_m_s = state.velocity_m_s - 3.130441649e-1;
        assert!(within(from_reference_m, -6.46e-8, 1.0e-2), "{from_reference_m:e}");
        assert!(within(from_reference_m_s, 1.66e-6, 1.0e-2), "{from_reference_m_s:e}");
        proof_intact(&frame);
    }

    // Spec test 2, second part, and the ledger over the scored ring-down.
    #[test]
    fn the_scored_ring_down_lands_through_its_last_block_and_goes_on_below_it() {
        let (beam, mut state) = scored_start();
        let mut frame = blank_frame();
        let mut worst = 0.0_f64;
        let mut landed = 0_u64;
        let mut first_below = None;
        // The last scored block ends 23.469 s after the start crossing, 93,876
        // steps after this start: every step of it lands inside the compared
        // envelope, and the decay goes on below it to 60 s.
        while landed < 240_000 {
            let before = state.envelope_m;
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            landed += 1;
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
            assert!(state.envelope_m <= COMPARED_CEILING_M);
            if state.envelope_m < AMPLITUDE_FLOOR_M {
                assert_eq!(frame.flags, FLAG_ENVELOPE_BELOW_COMPARED);
                if first_below.is_none() {
                    first_below = Some((landed, before));
                }
            } else {
                assert_eq!(frame.flags, 0);
            }
            if landed % 10 == 0 {
                proof_intact(&frame);
                assert_eq!(frame.pos[0], state.deflection_m as f32);
            }
        }
        let (first_flagged, envelope_before) = first_below.unwrap();
        println!(
            "from the start of the scored ring-down: first frame below the compared envelope at step {first_flagged}; after {landed} steps, at {} s, envelope {:e} m; largest ledger residual {worst:e}",
            state.t_s, state.envelope_m
        );
        // Through engine 0.9 the card refused the step after 94,136, at
        // 23.534 s, as the envelope fell below the floor.
        assert_eq!(first_flagged, 94_137);
        assert!(within(envelope_before, 5.8004e-5, 1.0e-4), "{envelope_before}");
        assert!(state.envelope_m < 1.0e-6, "{}", state.envelope_m);
        assert!(worst < RESIDUAL_CONSERVATION_LIMIT);
        assert!(within(worst, 7.368e-8, 1.0e-3), "{worst:e}");
        assert!(RESIDUAL_CONSERVATION_LIMIT > 13.0 * worst);

        // The whole compared run of 96,800 steps by the card's map and its
        // ledger, with no bound applied: it agrees with the reference to
        // 1e-9 and its largest residual is 5.6e-8, an eighteenth of the limit.
        let (worst, y) = ledger_on_map(
            &beam,
            &beam,
            [START_DEFLECTION_M, START_VELOCITY_M_S, 0.0],
            GATE_DT_S,
            96_800,
        );
        let differences = [
            relative(y[0], -5.251516710763e-5),
            relative(y[1], 4.826162861109e-4),
            relative(y[2], 3.214485022631e-1),
            relative(beam.stored_j_per_kg(y[0], y[1]), 3.815820888603e-6),
        ];
        println!(
            "after 96,800 steps of the map, relative difference from the reference: deflection {:e}, speed {:e}, heat {:e}, stored {:e}",
            differences[0], differences[1], differences[2], differences[3]
        );
        println!("largest ledger residual over the 96,800 steps of the compared run: {worst:e}");
        for difference in differences {
            assert!(difference <= 1.0e-9, "{difference:e}");
        }
        assert!(within(beam.envelope_m(y[0], y[1]), 5.332582e-5, 1.0e-6));
        assert!(within(worst, 5.624e-8, 1.0e-3), "{worst:e}");
        assert!(RESIDUAL_CONSERVATION_LIMIT > 17.0 * worst);

        // From rest at the largest envelope compared, 15.79 mm, the card's
        // ceiling through engine 0.11: it rings down through the compared
        // envelope and on toward rest, 100 s, with its books closed at every
        // step and no frame above the compared envelope.
        let mut state = CantileverState::try_phase(&beam, COMPARED_CEILING_M, 0.0).unwrap();
        let mut worst = 0.0_f64;
        let mut landed = 0_u64;
        let mut inside = 0_u64;
        while landed < 400_000 {
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            landed += 1;
            assert_eq!(frame.flags & FLAG_ENVELOPE_ABOVE_COMPARED, 0);
            if inside == 0 && state.envelope_m < AMPLITUDE_FLOOR_M {
                inside = landed - 1;
            }
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
        }
        println!(
            "from rest at the compared ceiling: {inside} steps before the first below the compared envelope, {landed} in all, envelope {:e} m; largest residual {worst:e}",
            state.envelope_m
        );
        assert_eq!(inside, 94_742);
        assert!(within(worst, 7.863e-8, 1.0e-3), "{worst:e}");
        assert!(RESIDUAL_CONSERVATION_LIMIT > 5.0 * worst);
        assert!(state.envelope_m < 1.0e-7 && state.envelope_m > 0.0);
        proof_intact(&frame);
    }

    // Spec test 3, and rule 10: a wrong stepping method is named.
    #[test]
    fn explicit_euler_is_refused_and_the_cards_own_step_lands() {
        let beam = CantileverEth::first_bending_mode();
        // From rest at the largest envelope compared, 15.79 mm.
        let mut state = CantileverState::try_phase(&beam, COMPARED_CEILING_M, 0.0).unwrap();
        let fresh = state;
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let scale = beam.ledger_scale_j_per_kg();

        // Where one Euler step of 0.25 ms lands, and what it does to the books.
        let landed = euler(&beam, state_vector(&state), GATE_DT_S).unwrap();
        assert_eq!(landed[0].to_bits(), COMPARED_CEILING_M.to_bits());
        let defect =
            beam.stored_j_per_kg(landed[0], landed[1]) - fresh.stored_j_per_kg + landed[2];
        assert!(within(landed[1], -1.006543233973e-2, 1.0e-9), "{:e}", landed[1]);
        assert!(within(defect, 5.065646e-5, 1.0e-6), "{defect:e}");
        assert!(within(defect / scale, 1.1187e-4, 1.0e-3));
        assert!(defect > 110.0 * RESIDUAL_CONSERVATION_LIMIT * scale);
        let got = step_with(&beam, &mut state, &mut frame, GATE_DT_S, euler);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), state_bits(&fresh));
        assert_eq!(frame.to_bytes(), held);

        // The card's own step from the same state.
        step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        assert!(within(state.deflection_m, 1.578874189283e-2, 1.0e-9));
        assert!(within(state.velocity_m_s, -1.006450254662e-2, 1.0e-9));
        assert!(within(state.heat_j_per_kg, 4.481389e-9, 1.0e-6));
        assert!(state.residual_conservation_error < 1.0e-13);
        proof_intact(&frame);

        // Euler from the start of the scored ring-down is refused on its first step.
        let (beam, mut state) = scored_start();
        let fresh = state;
        let mut frame = blank_frame();
        let got = step_with(&beam, &mut state, &mut frame, GATE_DT_S, euler);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), state_bits(&fresh));
        assert_eq!(frame.to_bytes(), held);

        // The midpoint rule at the ceiling lands for a while and is then
        // named. Found: 420 landed steps, then FirstLawBreach.
        let mut landed_steps = 0;
        let end = loop {
            match step_with(&beam, &mut state, &mut frame, GATE_DT_S, midpoint) {
                Ok(()) => landed_steps += 1,
                Err(end) => break end,
            }
        };
        println!("midpoint rule at the ceiling: {landed_steps} steps, then {end:?}");
        assert_eq!(end, StepReject::FirstLawBreach);
        assert!(landed_steps > 300 && landed_steps < 600, "{landed_steps}");

        // The card's own step, four times the ceiling: named by the ledger
        // through the public step. Found: one landed step, as the beam
        // reaches the corner of the damping term at zero deflection.
        let mut state = fresh;
        landed_steps = 0;
        let end = loop {
            match step(&beam, &mut state, &mut frame, NO_LOAD, 4.0 * GATE_DT_S) {
                Ok(()) => landed_steps += 1,
                Err(end) => break end,
            }
        };
        println!("the card's step at four times the ceiling: {landed_steps} steps, then {end:?}");
        assert_eq!(end, StepReject::FirstLawBreach);
        assert!(landed_steps >= 1 && landed_steps < 400, "{landed_steps}");
        // Twenty times the ceiling is refused on the first step.
        let mut state = fresh;
        assert_eq!(
            step(&beam, &mut state, &mut frame, NO_LOAD, 20.0 * GATE_DT_S),
            Err(StepReject::FirstLawBreach)
        );
        assert_eq!(state_bits(&state), state_bits(&fresh));

        // A step whose force has no softening, booked against the card's own
        // stored energy: the force is no longer the derivative of that energy.
        let mut no_softening = beam;
        no_softening.spring_kappa = 0.0;
        let mut state = fresh;
        let before = frame.to_bytes();
        let got = step_with(&beam, &mut state, &mut frame, GATE_DT_S, |_, y, dt_s| {
            rk4(&no_softening, y, dt_s)
        });
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), state_bits(&fresh));
        assert_eq!(frame.to_bytes(), before);
        proof_intact(&frame);
    }

    // Spec test 4, and rule 10: a falsified ledger is named.
    #[test]
    fn a_biased_heat_is_refused_both_ways_with_state_and_frame_bit_identical() {
        let (beam, mut state) = scored_start();
        let mut frame = blank_frame();
        let mut count = 0;
        while count < 100 {
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            count += 1;
        }
        assert!(state.heat_j_per_kg > 0.0);
        assert!(state.residual_conservation_error < 0.1 * RESIDUAL_CONSERVATION_LIMIT);
        let saved = state_bits(&state);
        let held = frame.to_bytes();
        let twice_the_limit = 2.0 * RESIDUAL_CONSERVATION_LIMIT * beam.ledger_scale_j_per_kg();

        let got = step_altered(&beam, &mut state, &mut frame, GATE_DT_S, |_, y| {
            y[2] += twice_the_limit
        });
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        let got = step_altered(&beam, &mut state, &mut frame, GATE_DT_S, |_, y| {
            y[2] -= twice_the_limit
        });
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);

        // Half of the step's heat booked, and one and a half times it.
        let got = step_altered(&beam, &mut state, &mut frame, GATE_DT_S, |held, y| {
            y[2] = held.heat_j_per_kg + 0.5 * (y[2] - held.heat_j_per_kg)
        });
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        let got = step_altered(&beam, &mut state, &mut frame, GATE_DT_S, |held, y| {
            y[2] = held.heat_j_per_kg + 1.5 * (y[2] - held.heat_j_per_kg)
        });
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        // The speed made larger or smaller by a thousandth.
        let got = step_altered(&beam, &mut state, &mut frame, GATE_DT_S, |_, y| y[1] *= 1.001);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        let got = step_altered(&beam, &mut state, &mut frame, GATE_DT_S, |_, y| y[1] *= 0.999);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);

        // A state whose ledger was altered between steps is named by the public step.
        let mut hot = state;
        hot.heat_j_per_kg += twice_the_limit;
        let hot_bits = state_bits(&hot);
        assert_eq!(
            step(&beam, &mut hot, &mut frame, NO_LOAD, GATE_DT_S),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(state_bits(&hot), hot_bits);
        let mut raised_anchor = state;
        raised_anchor.energy_anchor_j_per_kg += twice_the_limit;
        assert_eq!(
            step(&beam, &mut raised_anchor, &mut frame, NO_LOAD, GATE_DT_S),
            Err(StepReject::FirstLawBreach)
        );
        assert_eq!(frame.to_bytes(), held);

        // A bias of half the limit lands, and the faithful step lands.
        let mut biased = state;
        let half_the_limit = 0.25 * twice_the_limit;
        let got = step_altered(&beam, &mut biased, &mut frame, GATE_DT_S, |_, y| {
            y[2] += half_the_limit
        });
        assert_eq!(got, Ok(()));
        step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    // Spec test 5.
    #[test]
    fn the_two_amplitude_laws_are_in_the_step() {
        let beam = CantileverEth::first_bending_mode();
        let real = read_ring_down(&beam);
        // Period, amplitude mm, frequency Hz, decay rate per s:
        // `analysis/out/05_card_emulation.log`, TEST 4, rows 1, 4 and 7.
        let reference = [
            (0, 15.3249, 8.0518, 0.4517),
            (48, 2.1380, 8.1505, 0.2487),
            (160, 0.1210, 8.2266, 0.1838),
        ];
        for (period, amplitude_mm, frequency_hz, rate_per_s) in reference {
            let got = (
                real.amplitude_m[period] * 1.0e3,
                real.frequency_hz(period),
                real.decay_rate_per_s(period),
            );
            println!(
                "period {period}: amplitude {:.4} mm, frequency {:.4} Hz, decay rate {:.4} per s",
                got.0, got.1, got.2
            );
            assert!((got.0 - amplitude_mm).abs() < 1.0e-4, "amplitude {}", got.0);
            assert!((got.1 - frequency_hz).abs() < 1.0e-4, "frequency {}", got.1);
            assert!((got.2 - rate_per_s).abs() < 1.0e-4, "rate {}", got.2);
        }
        // The frequency rises by 0.175 Hz and the decay rate falls by a factor
        // of 2.5 between the largest and the smallest of the three amplitudes.
        assert!(real.frequency_hz(160) - real.frequency_hz(0) > 0.17);
        assert!(real.decay_rate_per_s(0) > 2.4 * real.decay_rate_per_s(160));

        // The softening dropped: one frequency throughout, the frequency at
        // vanishing amplitude.
        let mut altered = beam;
        altered.spring_kappa = 0.0;
        let reading = read_ring_down(&altered);
        for period in [0, 48, 160] {
            assert!(
                (reading.frequency_hz(period) - SMALL_AMPLITUDE_FREQUENCY_HZ).abs() < 5.0e-4,
                "{}",
                reading.frequency_hz(period)
            );
        }
        // The drag and the damping power dropped: one decay rate throughout, c / 2.
        let mut altered = beam;
        altered.drag_per_m = 0.0;
        altered.damping_power = 0.0;
        let reading = read_ring_down(&altered);
        for period in [0, 48, 160] {
            assert!(
                (reading.decay_rate_per_s(period) - 0.5 * DAMPING_C).abs() < 2.0e-3,
                "{}",
                reading.decay_rate_per_s(period)
            );
        }
        // The drag alone dropped: the decay rate at the largest amplitude is
        // 0.244 per second in place of 0.452.
        let mut no_drag = beam;
        no_drag.drag_per_m = 0.0;
        let reading = read_ring_down(&no_drag);
        assert!((reading.decay_rate_per_s(0) - 0.2437).abs() < 1.0e-3);
        // The damping power alone dropped: the decay rate at the smallest
        // amplitude is 0.337 per second in place of 0.184.
        let mut altered = beam;
        altered.damping_power = 0.0;
        let reading = read_ring_down(&altered);
        assert!((reading.decay_rate_per_s(160) - 0.3371).abs() < 1.0e-3);

        // What the ledger cannot see. A step with the drag dropped from both
        // the force and the heat balances its books for the whole run: the
        // drag is not part of the stored energy. The reading above is what
        // tells that step from the card's. Measured on the map from the start
        // of the compared run, with no bound applied.
        let (worst, _) = ledger_on_map(
            &beam,
            &no_drag,
            [START_DEFLECTION_M, START_VELOCITY_M_S, 0.0],
            GATE_DT_S,
            96_000,
        );
        assert!(worst < RESIDUAL_CONSERVATION_LIMIT);
        // And through the card's own step from the start of the scored
        // ring-down, through the compared envelope and below it.
        let (_, mut state) = scored_start();
        let mut frame = blank_frame();
        let mut landed = 0;
        while landed < 120_000 {
            step_with(&beam, &mut state, &mut frame, GATE_DT_S, |_, y, dt_s| {
                rk4(&no_drag, y, dt_s)
            })
            .unwrap();
            landed += 1;
            assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
        }
        println!("the card's step with the drag dropped: {landed} steps landed");
        proof_intact(&frame);
    }

    // Spec test 6.
    #[test]
    fn zero_deflection_is_safe() {
        // The power of zero, of minus zero and of a subnormal number is
        // exactly zero for each exponent the card uses.
        let exponents = [DAMPING_POWER, SPRING_POWER - 1.0, SPRING_POWER, SPRING_POWER + 1.0];
        let smallest_normal = f64::MIN_POSITIVE;
        for exponent in exponents {
            assert_eq!(raised(0.0, exponent).to_bits(), 0.0_f64.to_bits());
            assert_eq!(raised(-0.0, exponent).to_bits(), 0.0_f64.to_bits());
            assert_eq!(raised(f64::from_bits(1), exponent).to_bits(), 0.0_f64.to_bits());
            assert_eq!(raised(0.5 * smallest_normal, exponent).to_bits(), 0.0_f64.to_bits());
            let at_smallest_normal = raised(smallest_normal, exponent);
            assert!(at_smallest_normal.is_finite() && at_smallest_normal >= 0.0);
            assert!(raised(1.0e-300, exponent).is_finite());
            assert!(raised(f64::NAN, exponent).is_nan());
        }
        assert!(raised(smallest_normal, DAMPING_POWER) > 0.0);
        assert_eq!(raised(1.0, SPRING_POWER).to_bits(), 1.0_f64.to_bits());

        let beam = CantileverEth::first_bending_mode();
        let mut frame = blank_frame();
        for velocity_m_s in [0.0, -0.0, 1.0e-300, 1.0e-6, -1.0e-6, 0.5, -0.5, 0.75, -0.75] {
            for deflection_m in [0.0, -0.0] {
                // At zero deflection only the air drag acts.
                let rate = rates(&beam, [deflection_m, velocity_m_s, 0.0]).unwrap();
                let speed = velocity_m_s.abs();
                assert_eq!(rate[0].to_bits(), velocity_m_s.to_bits());
                assert_eq!(rate[1], -(DRAG_PER_M * speed * velocity_m_s));
                assert_eq!(rate[2], DRAG_PER_M * (speed * speed * speed));

                // The map is finite there, and at rest at zero nothing happens.
                let y = rk4(&beam, [deflection_m, velocity_m_s, 0.0], GATE_DT_S).unwrap();
                assert!(y[0].is_finite() && y[1].is_finite());
                assert!(y[2].is_finite() && y[2] >= 0.0);
                if velocity_m_s == 0.0 {
                    assert_eq!(y, [0.0, 0.0, 0.0]);
                }

                // Under the compared envelope, rest included, the card makes a
                // start and steps it, with the flag set.
                let mut state =
                    CantileverState::try_phase(&beam, deflection_m, velocity_m_s).unwrap();
                assert_eq!(state.potential_j_per_kg, 0.0);
                step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
                assert!(state.deflection_m.is_finite());
                assert!(state.velocity_m_s.is_finite());
                assert!(state.heat_j_per_kg.is_finite() && state.heat_j_per_kg >= 0.0);
                assert!(state.stored_j_per_kg.is_finite());
                assert!(state.envelope_m.is_finite());
                assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                assert_eq!(frame.t.to_bits(), GATE_DT_S.to_bits());
                if beam.envelope_m(deflection_m, velocity_m_s) < AMPLITUDE_FLOOR_M {
                    assert_eq!(frame.flags, FLAG_ENVELOPE_BELOW_COMPARED);
                }
                if velocity_m_s == 0.0 {
                    // Rest at zero is a state that does not move.
                    assert_eq!(state.deflection_m, 0.0);
                    assert_eq!(state.velocity_m_s, 0.0);
                    assert_eq!(state.heat_j_per_kg, 0.0);
                    assert_eq!(state.residual_conservation_error, 0.0);
                }
            }
        }
        proof_intact(&frame);
    }

    // Spec test 7, and every other refusal.
    #[test]
    fn every_refusal_is_named_and_leaves_state_and_frame_bit_identical() {
        let (beam, fresh) = scored_start();
        let ok = GATE_DT_S;

        // A start above the ceiling is not made, on either side of zero.
        assert!(CantileverState::try_phase(&beam, 0.0189, 0.0).is_none());
        assert!(CantileverState::try_phase(&beam, -0.0189, 0.0).is_none());
        assert!(CantileverState::try_phase(&beam, 0.01876, 0.0).is_none());
        assert!(CantileverState::try_phase(&beam, -0.01876, 0.0).is_none());
        assert!(CantileverState::try_phase(&beam, 0.0, 0.9893).is_none());
        assert!(CantileverState::try_phase(&beam, 0.0, -0.9893).is_none());
        assert!(CantileverState::try_phase(&beam, 0.0, 0.977).is_none());
        // Above the compared envelope and under the ceiling a start is made:
        // through engine 0.11 these were refused.
        assert!(CantileverState::try_phase(&beam, 0.0158, 0.0).is_some());
        assert!(CantileverState::try_phase(&beam, -0.0158, 0.0).is_some());
        assert!(CantileverState::try_phase(&beam, 0.0, 0.823).is_some());
        assert!(CantileverState::try_phase(&beam, 0.0, -0.976).is_some());
        // Below the compared envelope a start is made, rest at zero included.
        assert!(CantileverState::try_phase(&beam, 0.0, 0.0).is_some());
        assert!(CantileverState::try_phase(&beam, 5.7e-5, 0.0).is_some());
        assert!(CantileverState::try_phase(&beam, 0.0, -3.0e-3).is_some());
        assert!(CantileverState::try_phase(&beam, f64::NAN, 0.0).is_none());
        assert!(CantileverState::try_phase(&beam, 0.0, f64::NAN).is_none());
        assert!(CantileverState::try_phase(&beam, f64::INFINITY, 0.0).is_none());
        assert!(CantileverState::try_phase(&beam, 0.0, f64::NEG_INFINITY).is_none());
        // The ceiling itself and the floor itself are made.
        assert!(CantileverState::try_phase(&beam, AMPLITUDE_CEILING_M, 0.0).is_some());
        assert!(CantileverState::try_phase(&beam, -AMPLITUDE_CEILING_M, 0.0).is_some());
        assert!(CantileverState::try_phase(&beam, AMPLITUDE_FLOOR_M, 0.0).is_some());
        assert!(CantileverState::try_phase(&beam, -AMPLITUDE_FLOOR_M, 0.0).is_some());

        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let mut refuse =
            |mut state: CantileverState, dt_s: f64, card: &CantileverEth, expect: StepReject| {
                let saved = state_bits(&state);
                let got = step(card, &mut state, &mut frame, NO_LOAD, dt_s);
                assert_eq!(got, Err(expect), "dt {dt_s}");
                assert_eq!(state_bits(&state), saved);
                assert_eq!(frame.to_bytes(), held);
                proof_intact(&frame);
            };

        refuse(fresh, f64::NAN, &beam, StepReject::NonFiniteInput);
        refuse(fresh, f64::INFINITY, &beam, StepReject::NonFiniteInput);
        refuse(fresh, f64::NEG_INFINITY, &beam, StepReject::NonFiniteInput);
        refuse(fresh, 0.0, &beam, StepReject::NonPositiveDt);
        refuse(fresh, -0.0, &beam, StepReject::NonPositiveDt);
        refuse(fresh, -ok, &beam, StepReject::NonPositiveDt);
        refuse(fresh, -f64::MAX, &beam, StepReject::NonPositiveDt);

        let mut broken = fresh;
        broken.deflection_m = f64::NAN;
        refuse(broken, ok, &beam, StepReject::NonFiniteState);
        broken = fresh;
        broken.velocity_m_s = f64::INFINITY;
        refuse(broken, ok, &beam, StepReject::NonFiniteState);
        broken = fresh;
        broken.heat_j_per_kg = f64::NAN;
        refuse(broken, ok, &beam, StepReject::NonFiniteState);
        broken = fresh;
        broken.energy_anchor_j_per_kg = f64::NEG_INFINITY;
        refuse(broken, ok, &beam, StepReject::NonFiniteState);
        broken = fresh;
        broken.t_s = f64::INFINITY;
        refuse(broken, ok, &beam, StepReject::NonFiniteState);
        broken = fresh;
        broken.residual_conservation_error = f64::NAN;
        refuse(broken, ok, &beam, StepReject::NonFiniteState);
        // A stage of the trial step that is not finite.
        refuse(fresh, f64::MAX, &beam, StepReject::NonFiniteState);
        refuse(fresh, 1.0e200, &beam, StepReject::NonFiniteState);

        let mut tampered = beam;
        tampered.drag_per_m = 0.0;
        refuse(fresh, ok, &tampered, StepReject::InvalidBeam);
        tampered = beam;
        tampered.amplitude_ceiling_m = 1.0;
        refuse(fresh, ok, &tampered, StepReject::InvalidBeam);

        // A state above the ceiling, put together by hand. The recorded
        // envelope is left as it was: the card works it out again.
        let mut high = fresh;
        high.deflection_m = 0.0189;
        high.velocity_m_s = 0.0;
        refuse(high, ok, &beam, StepReject::EnvelopeAboveCard);
        high.deflection_m = 0.0;
        high.velocity_m_s = 0.9893;
        refuse(high, ok, &beam, StepReject::EnvelopeAboveCard);
        high.deflection_m = 1.0;
        refuse(high, ok, &beam, StepReject::EnvelopeAboveCard);
        // A state below the compared envelope lands, rest included: the
        // bound that stood there through engine 0.9 is gone.
        let mut low = fresh;
        low.deflection_m = 1.0e-5;
        low.velocity_m_s = 0.0;
        low.energy_anchor_j_per_kg = beam.stored_j_per_kg(1.0e-5, 0.0);
        low.heat_j_per_kg = 0.0;
        let mut landed = low;
        step(&beam, &mut landed, &mut frame, NO_LOAD, ok).unwrap();
        assert_eq!(frame.flags, FLAG_ENVELOPE_BELOW_COMPARED);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let mut refuse =
            |mut state: CantileverState, dt_s: f64, card: &CantileverEth, expect: StepReject| {
                let saved = state_bits(&state);
                let got = step(card, &mut state, &mut frame, NO_LOAD, dt_s);
                assert_eq!(got, Err(expect), "dt {dt_s}");
                assert_eq!(state_bits(&state), saved);
                assert_eq!(frame.to_bytes(), held);
                proof_intact(&frame);
            };

        // The ledger, through the public step: a step twenty times the ceiling.
        refuse(fresh, 20.0 * ok, &beam, StepReject::FirstLawBreach);
        // And a heat altered between steps.
        let mut hot = fresh;
        hot.heat_j_per_kg = 1.0e-5;
        refuse(hot, ok, &beam, StepReject::SpontaneousGeneration);
        let mut cold = fresh;
        cold.heat_j_per_kg = -1.0e-5;
        refuse(cold, ok, &beam, StepReject::FirstLawBreach);

        // The order of the checks: the step size, the state, the sign of the
        // step, the card, the ceiling.
        broken = fresh;
        broken.deflection_m = f64::NAN;
        refuse(broken, f64::NAN, &tampered, StepReject::NonFiniteInput);
        refuse(broken, 0.0, &tampered, StepReject::NonFiniteState);
        refuse(high, 0.0, &tampered, StepReject::NonPositiveDt);
        refuse(high, ok, &tampered, StepReject::InvalidBeam);
        refuse(low, ok, &tampered, StepReject::InvalidBeam);

        // The same step lands on the fresh state.
        let mut state = fresh;
        step(&beam, &mut state, &mut frame, NO_LOAD, ok).unwrap();
        assert!(frame.to_bytes() != held);
        proof_intact(&frame);
    }

    #[test]
    fn a_start_at_zero_deflection_near_the_ceiling_is_refused_when_its_envelope_passes_it() {
        // The envelope is measured with the frequency at vanishing amplitude
        // and the softened spring is weaker, so the envelope rises between the
        // rest position and the turning point.
        let beam = CantileverEth::first_bending_mode();
        let speed_at_ceiling_m_s = AMPLITUDE_CEILING_M * beam.omega0_rad_s();
        let mut frame = blank_frame();

        let mut state = CantileverState::try_phase(&beam, 0.0, 0.99 * speed_at_ceiling_m_s).unwrap();
        let mut landed = 0;
        let end = loop {
            let saved = state_bits(&state);
            let held = frame.to_bytes();
            match step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S) {
                Ok(()) => landed += 1,
                Err(end) => {
                    assert_eq!(state_bits(&state), saved);
                    assert_eq!(frame.to_bytes(), held);
                    break end;
                }
            }
        };
        println!(
            "start at zero deflection, envelope 0.99 of the ceiling: {landed} steps, then {end:?} at deflection {:e} m",
            state.deflection_m
        );
        assert_eq!(end, StepReject::EnvelopeAboveCard);
        // Found: 107 steps, a little under a quarter of a period (120 steps).
        assert_eq!(landed, 107);
        assert!(state.envelope_m <= AMPLITUDE_CEILING_M);
        assert!(state.deflection_m > 0.0138 && state.deflection_m < AMPLITUDE_CEILING_M);
        // The last landed frame is above the compared envelope: it carries the
        // flag and a margin to the largest envelope compared below zero.
        assert_eq!(frame.flags, FLAG_ENVELOPE_ABOVE_COMPARED);
        assert!(frame.force_torque < 0.0);
        proof_intact(&frame);

        // At 0.98 of the ceiling the beam rings through.
        let mut state = CantileverState::try_phase(&beam, 0.0, 0.98 * speed_at_ceiling_m_s).unwrap();
        let mut top_m = 0.0_f64;
        let mut count = 0;
        while count < 2_000 {
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            if state.envelope_m > top_m {
                top_m = state.envelope_m;
            }
            count += 1;
        }
        println!("start at zero deflection, envelope 0.98 of the ceiling: rings through, the envelope tops at {} of the ceiling", top_m / AMPLITUDE_CEILING_M);
        // The envelope tops at 0.991 of it.
        assert!(top_m > 0.991 * AMPLITUDE_CEILING_M && top_m < 0.992 * AMPLITUDE_CEILING_M, "{top_m}");
    }

    // The wall at the largest envelope compared, lifted: from 15.79 mm to
    // 18.75 mm, the largest amplitude the constants were fitted on, the card
    // answers with the same equations and the same ledger, and each frame
    // there carries FLAG_ENVELOPE_ABOVE_COMPARED; above 18.75 mm it refuses
    // by name, as EnvelopeAboveCard.
    #[test]
    fn above_the_compared_envelope_a_step_lands_flagged_with_its_books_closed_up_to_the_ceiling() {
        let beam = CantileverEth::first_bending_mode();
        let mut frame = blank_frame();
        let limit = RESIDUAL_CONSERVATION_LIMIT;

        // Starts between the two are made, and their first step lands flagged,
        // with the margin to the largest envelope compared below zero.
        for (deflection_m, velocity_m_s) in [
            (0.0158, 0.0),
            (-0.0170, 0.0),
            (0.0185, 0.0),
            (AMPLITUDE_CEILING_M, 0.0),
            (-AMPLITUDE_CEILING_M, 0.0),
            (0.012, 0.6),
            (0.0, -0.95),
        ] {
            let envelope_m = beam.envelope_m(deflection_m, velocity_m_s);
            assert!(envelope_m > COMPARED_CEILING_M && envelope_m <= AMPLITUDE_CEILING_M, "{envelope_m}");
            let mut state = CantileverState::try_phase(&beam, deflection_m, velocity_m_s).unwrap();
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            assert_eq!(frame.flags, FLAG_ENVELOPE_ABOVE_COMPARED, "{deflection_m} {velocity_m_s}");
            assert!(frame.force_torque < 0.0);
            assert_eq!(frame.force_torque, (COMPARED_CEILING_M - state.envelope_m) as f32);
            assert!(state.residual_conservation_error <= limit);
            proof_intact(&frame);
        }

        // From rest at the ceiling, 18.75 mm: the ring-down lands through the
        // flagged envelopes, through the compared envelope with every frame
        // unflagged, and below it toward rest, 100 s, its books closed at
        // every step. The flag is on exactly the frames above 15.79 mm; the
        // envelope rises within each period toward the turning point, so
        // near 15.79 mm flagged and unflagged frames alternate within a
        // period before the decay takes the envelope under it for good.
        let mut state = CantileverState::try_phase(&beam, AMPLITUDE_CEILING_M, 0.0).unwrap();
        let mut worst = 0.0_f64;
        let mut worst_above = 0.0_f64;
        let mut landed = 0_u64;
        let mut above = 0_u64;
        let mut first_unflagged = 0_u64;
        let mut last_flagged = 0_u64;
        let mut first_below = 0_u64;
        let mut heat = 0.0_f64;
        while landed < 400_000 {
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            landed += 1;
            assert!(state.envelope_m <= AMPLITUDE_CEILING_M);
            assert!(state.heat_j_per_kg >= heat);
            heat = state.heat_j_per_kg;
            if state.envelope_m > COMPARED_CEILING_M {
                assert_eq!(frame.flags, FLAG_ENVELOPE_ABOVE_COMPARED);
                above += 1;
                last_flagged = landed;
                if state.residual_conservation_error > worst_above {
                    worst_above = state.residual_conservation_error;
                }
            } else if state.envelope_m < AMPLITUDE_FLOOR_M {
                assert_eq!(frame.flags, FLAG_ENVELOPE_BELOW_COMPARED);
                if first_below == 0 {
                    first_below = landed;
                }
            } else {
                assert_eq!(frame.flags, 0);
                if first_unflagged == 0 {
                    first_unflagged = landed;
                }
            }
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
        }
        println!(
            "from rest at the ceiling, 18.75 mm: {above} frames above the compared envelope, the first inside it at step {first_unflagged}, the last above it at step {last_flagged} ({} s), the first below it at step {first_below}; {landed} in all, envelope {:e} m; largest residual {worst:e}, {worst_above:e} above the compared envelope",
            last_flagged as f64 * GATE_DT_S,
            state.envelope_m
        );
        // Found: 1,149 frames above 15.79 mm, the last at step 1,269,
        // 0.317 s; the first below 0.058 mm at step 96,004; the largest
        // residual 1.044e-7, a tenth of the limit, and 6.33e-8 above the
        // compared envelope.
        assert_eq!((above, last_flagged, first_below), (1_149, 1_269, 96_004));
        assert!(first_unflagged > 0 && first_unflagged < last_flagged);
        assert!(within(worst, 1.044e-7, 1.0e-3), "{worst:e}");
        assert!(within(worst_above, 6.329e-8, 1.0e-3), "{worst_above:e}");
        assert!(worst < limit);
        assert!(state.envelope_m < 1.0e-7 && state.envelope_m > 0.0);
        proof_intact(&frame);

        // The ledger above the compared envelope is the ledger: a heat altered
        // there, a wrong step and a force that is not the derivative of the
        // stored energy are named, with the state and the frame as they were.
        let high = CantileverState::try_phase(&beam, 0.0185, 0.0).unwrap();
        let gate = limit * beam.ledger_scale_j_per_kg();
        let held = frame.to_bytes();
        for (bias, expect) in [
            (2.0 * gate, StepReject::SpontaneousGeneration),
            (-2.0 * gate, StepReject::FirstLawBreach),
        ] {
            let mut altered = high;
            altered.heat_j_per_kg += bias;
            let saved = state_bits(&altered);
            assert_eq!(step(&beam, &mut altered, &mut frame, NO_LOAD, GATE_DT_S), Err(expect));
            assert_eq!(state_bits(&altered), saved);
            assert_eq!(frame.to_bytes(), held);
        }
        let mut state = high;
        assert_eq!(step_with(&beam, &mut state, &mut frame, GATE_DT_S, euler), Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), state_bits(&high));
        let mut no_softening = beam;
        no_softening.spring_kappa = 0.0;
        assert_eq!(
            step_with(&beam, &mut state, &mut frame, GATE_DT_S, |_, y, dt_s| rk4(&no_softening, y, dt_s)),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(state_bits(&state), state_bits(&high));
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        // Above the ceiling a start is not made, and a state put together by
        // hand there is refused by name, with the state and the frame as they
        // were; the next 64-bit number above the ceiling is refused, the
        // ceiling itself is made.
        let next = f64::from_bits(AMPLITUDE_CEILING_M.to_bits() + 1);
        assert!(CantileverState::try_phase(&beam, next, 0.0).is_none());
        assert!(CantileverState::try_phase(&beam, -next, 0.0).is_none());
        assert!(CantileverState::try_phase(&beam, AMPLITUDE_CEILING_M, 0.0).is_some());
        for (deflection_m, velocity_m_s) in [(next, 0.0), (0.0188, 0.0), (-0.0190, 0.0), (0.0, 0.98), (0.015, 0.6)] {
            assert!(beam.envelope_m(deflection_m, velocity_m_s) > AMPLITUDE_CEILING_M);
            let mut state = high;
            state.deflection_m = deflection_m;
            state.velocity_m_s = velocity_m_s;
            let saved = state_bits(&state);
            assert_eq!(step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S), Err(StepReject::EnvelopeAboveCard));
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
        }
        proof_intact(&frame);
    }

    // Spec test 8, the wall lifted: below the smallest envelope the card was
    // compared at it goes on with the same equations and ledger, and each
    // frame there carries FLAG_ENVELOPE_BELOW_COMPARED.
    #[test]
    fn a_ring_down_goes_on_below_the_compared_envelope_to_rest_with_its_books_closed() {
        let beam = CantileverEth::first_bending_mode();
        let mut frame = blank_frame();
        assert_eq!(frame.flags, 9);

        // Under 0.058 mm a start is made, rest at zero included, and its step
        // carries the flag.
        for (deflection_m, velocity_m_s) in [(5.7e-5, 0.0), (-5.0e-5, 0.0), (0.0, 3.0e-3), (0.0, 0.0)] {
            assert!(beam.envelope_m(deflection_m, velocity_m_s) < AMPLITUDE_FLOOR_M);
            let mut state = CantileverState::try_phase(&beam, deflection_m, velocity_m_s).unwrap();
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            assert_eq!(frame.flags, FLAG_ENVELOPE_BELOW_COMPARED);
        }
        // At and above it a step lands and the frame's flags are zero.
        for (deflection_m, velocity_m_s) in [(6.5e-5, 0.0), (1.0e-3, 0.0), (0.0, 0.5), (COMPARED_CEILING_M, 0.0)] {
            let mut state = CantileverState::try_phase(&beam, deflection_m, velocity_m_s).unwrap();
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            assert!(state.envelope_m >= AMPLITUDE_FLOOR_M);
            assert_eq!(frame.flags, 0);
        }
        // A ring-down from 0.1 mm passes under the old floor and decays on for
        // 200 s, 800,000 steps: the envelope never rises by more than the
        // softened spring's rise within one period, the heat never falls, the
        // ledger closes at every step, and every value stays finite and normal
        // or exactly zero.
        let mut state = CantileverState::try_phase(&beam, 1.0e-4, 0.0).unwrap();
        let mut landed = 0_u64;
        let mut flagged = 0_u64;
        let mut worst = 0.0_f64;
        let mut heat = 0.0_f64;
        while landed < 800_000 {
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            landed += 1;
            if frame.flags == FLAG_ENVELOPE_BELOW_COMPARED {
                flagged += 1;
                assert!(state.envelope_m < AMPLITUDE_FLOOR_M);
            } else {
                assert_eq!(frame.flags, 0);
                assert!(state.envelope_m >= AMPLITUDE_FLOOR_M);
            }
            assert!(state.heat_j_per_kg >= heat);
            heat = state.heat_j_per_kg;
            assert!(state.envelope_m.is_finite() && state.envelope_m >= 0.0);
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
        }
        println!(
            "ring-down from 0.1 mm: {landed} steps, {flagged} below the compared envelope, envelope {:e} m, largest residual {worst:e}",
            state.envelope_m
        );
        assert!(flagged > 780_000);
        assert!(worst < 1.0e-3 * RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
        assert!(state.envelope_m < 1.0e-8, "{:e}", state.envelope_m);
        proof_intact(&frame);

        // The ledger past the old floor is the ledger: a heat altered there is
        // named, both ways, with the state and the frame as they were.
        let gate = RESIDUAL_CONSERVATION_LIMIT * beam.ledger_scale_j_per_kg();
        for (bias, expect) in [
            (2.0 * gate, StepReject::SpontaneousGeneration),
            (-2.0 * gate, StepReject::FirstLawBreach),
        ] {
            let mut altered = state;
            altered.heat_j_per_kg += bias;
            let saved = state_bits(&altered);
            let held = frame.to_bytes();
            assert_eq!(step(&beam, &mut altered, &mut frame, NO_LOAD, GATE_DT_S), Err(expect));
            assert_eq!(state_bits(&altered), saved);
            assert_eq!(frame.to_bytes(), held);
        }
        // And explicit Euler from just under the old floor creates energy,
        // which the ledger names.
        let small = CantileverState::try_phase(&beam, 5.0e-5, 0.0).unwrap();
        let mut euler_state = small;
        let mut named = None;
        let mut count = 0;
        while count < 100_000 {
            match step_with(&beam, &mut euler_state, &mut frame, GATE_DT_S, |card, y, dt_s| {
                euler(card, y, dt_s)
            }) {
                Ok(()) => count += 1,
                Err(end) => {
                    named = Some(end);
                    break;
                }
            }
        }
        println!("explicit Euler from 0.05 mm: {count} steps, then {named:?}");
        assert_eq!(named, Some(StepReject::SpontaneousGeneration));
        assert_eq!(count, 781);
    }

    // Spec test 9.
    #[test]
    fn a_tiny_step_is_not_clamped() {
        let beam = CantileverEth::first_bending_mode();
        let mut state = CantileverState::try_phase(&beam, 1.0e-3, 0.0).unwrap();
        let mut frame = blank_frame();
        step(&beam, &mut state, &mut frame, NO_LOAD, 1.0e-15).unwrap();
        assert!(within(state.velocity_m_s, -2.635568e-15, 1.0e-6), "{:e}", state.velocity_m_s);
        assert!(state.velocity_m_s != 0.0);
        assert_eq!(state.deflection_m.to_bits(), 1.0e-3_f64.to_bits());
        assert_eq!(state.t_s.to_bits(), 1.0e-15_f64.to_bits());
        assert_eq!(frame.t.to_bits(), 1.0e-15_f64.to_bits());
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        // A step raised to 1e-12 s would have given a speed a thousand times larger.
        assert!(state.velocity_m_s.abs() < 1.0e-14);
        proof_intact(&frame);
    }

    #[test]
    fn the_frame_carries_the_declared_slots() {
        let (beam, mut state) = scored_start();
        let mut frame = blank_frame();
        let mut count = 0;
        while count < 37 {
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            count += 1;
        }
        assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
        assert_eq!(frame.pos[0], state.deflection_m as f32);
        assert_eq!(frame.pos[1], state.velocity_m_s as f32);
        assert_eq!(frame.pos[2], state.envelope_m as f32);
        assert_eq!(frame.vel[0], state.kinetic_j_per_kg as f32);
        assert_eq!(frame.vel[1], state.potential_j_per_kg as f32);
        assert_eq!(frame.vel[2], state.heat_j_per_kg as f32);
        assert_eq!(frame.force_torque, (COMPARED_CEILING_M - state.envelope_m) as f32);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        assert_eq!(frame.flags, 0);
        proof_intact(&frame);
        // The state's own books agree with the card's functions.
        assert_eq!(
            state.kinetic_j_per_kg.to_bits(),
            (0.5 * state.velocity_m_s * state.velocity_m_s).to_bits()
        );
        assert_eq!(
            state.potential_j_per_kg.to_bits(),
            beam.potential_j_per_kg(state.deflection_m).to_bits()
        );
        assert_eq!(
            state.stored_j_per_kg.to_bits(),
            beam.stored_j_per_kg(state.deflection_m, state.velocity_m_s).to_bits()
        );
        assert_eq!(
            state.envelope_m.to_bits(),
            beam.envelope_m(state.deflection_m, state.velocity_m_s).to_bits()
        );
        assert_eq!(
            state.energy_defect_j_per_kg.to_bits(),
            (state.stored_j_per_kg - state.energy_anchor_j_per_kg + state.heat_j_per_kg).to_bits()
        );
        assert!(state.heat_j_per_kg > 0.0);
        assert!(state.stored_j_per_kg < state.energy_anchor_j_per_kg);
    }

    #[test]
    fn the_proof_is_not_the_cards() {
        let (beam, mut state) = scored_start();
        let mut frame = blank_frame();
        let mut count = 0;
        while count < 500 {
            step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            proof_intact(&frame);
            count += 1;
        }
        assert_eq!(
            step(&beam, &mut state, &mut frame, NO_LOAD, 0.0),
            Err(StepReject::NonPositiveDt)
        );
        proof_intact(&frame);
        assert_eq!(
            step(&beam, &mut state, &mut frame, NO_LOAD, f64::NAN),
            Err(StepReject::NonFiniteInput)
        );
        proof_intact(&frame);
        assert!(step_with(&beam, &mut state, &mut frame, GATE_DT_S, euler).is_err());
        proof_intact(&frame);
        assert!(step_altered(&beam, &mut state, &mut frame, GATE_DT_S, |_, y| y[2] += 1.0).is_err());
        proof_intact(&frame);
        let mut tampered = beam;
        tampered.spring_power = 3.0;
        assert_eq!(
            step(&tampered, &mut state, &mut frame, NO_LOAD, GATE_DT_S),
            Err(StepReject::InvalidBeam)
        );
        proof_intact(&frame);
        // A proof put there by the sealing crate stays as it was written.
        let mut index = 0;
        while index < 16 {
            frame.proof[index] = index as u8;
            index += 1;
        }
        step(&beam, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        index = 0;
        while index < 16 {
            assert_eq!(frame.proof[index], index as u8);
            index += 1;
        }
    }

    #[test]
    fn the_step_ceiling_and_floor_are_present_and_derived() {
        let beam = CantileverEth::first_bending_mode();
        let ceiling_s = beam.gate_dt_s().unwrap();
        let floor_s = beam.floor_dt_s().unwrap();
        assert_eq!(ceiling_s.to_bits(), 2.5e-4_f64.to_bits());
        assert_eq!(floor_s.to_bits(), (2.5e-4_f64 / 262_144.0).to_bits());
        assert!(within(floor_s, 9.536743e-10, 1.0e-6));
        assert!(floor_s < ceiling_s);

        // The ceiling: about 482 steps per period, ω0 dt = 0.013, and ten
        // steps make one frame at 400 per second.
        let steps_per_period = 1.0 / (SMALL_AMPLITUDE_FREQUENCY_HZ * ceiling_s);
        assert!(steps_per_period > 482.0 && steps_per_period < 483.0, "{steps_per_period}");
        let phase_step_rad = beam.omega0_rad_s() * ceiling_s;
        assert!(phase_step_rad > 0.0130 && phase_step_rad < 0.0131);
        assert!(within(10.0 * ceiling_s, 1.0 / 400.0, 1.0e-15));
        // The measured ground of the ceiling, the ledger at an eighteenth of
        // its limit over the compared run, is the test
        // `the_scored_ring_down_lands_through_its_last_block_and_goes_on_below_it`;
        // the table of the header is the test
        // `halving_the_step_shows_the_slow_convergence_of_the_table`.

        // A broken card has neither.
        let mut broken = beam;
        broken.gate_dt_s = 0.0;
        assert!(broken.gate_dt_s().is_none());
        assert!(broken.floor_dt_s().is_none());
        broken.gate_dt_s = f64::NAN;
        assert!(broken.gate_dt_s().is_none());
        assert!(broken.floor_dt_s().is_none());
        broken = beam;
        broken.gate_over_floor = 0.0;
        assert!(broken.floor_dt_s().is_none());

        // The floor: from rest at the amplitude ceiling, a turning point,
        // where the deflection moves least, one step at the floor moves the
        // deflection by six units in the last place, toward rest (five from
        // rest at 15.79 mm, the ceiling through engine 0.11).
        let phase_at_floor_rad = beam.omega0_rad_s() * floor_s;
        assert!(phase_at_floor_rad > 4.9e-8 && phase_at_floor_rad < 5.0e-8);
        for sign in [1.0, -1.0] {
            let at_rest = CantileverState::try_phase(&beam, sign * AMPLITUDE_CEILING_M, 0.0).unwrap();
            let mut state = at_rest;
            let mut frame = blank_frame();
            step(&beam, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
            assert_eq!(at_rest.deflection_m.abs().to_bits() - state.deflection_m.abs().to_bits(), 6);
            assert!(state.envelope_m <= AMPLITUDE_CEILING_M);
            // No step from one to four floors is refused there, over four
            // steps each; at the earlier floor, a quarter of this one, steps
            // of one to four of it were refused as EnvelopeAboveCard by
            // rounding, the deflection standing still while the speed grew.
            let mut refused_now = 0;
            let mut refused_before = 0;
            let mut index = 0;
            while index < 2_000 {
                let multiple = 1.0 + 3.0 * (index as f64) / 1_999.0;
                for (dt_s, count) in [
                    (multiple * floor_s, &mut refused_now),
                    (0.25 * multiple * floor_s, &mut refused_before),
                ] {
                    let mut state = at_rest;
                    let mut steps = 0;
                    while steps < 4 {
                        match step(&beam, &mut state, &mut frame, NO_LOAD, dt_s) {
                            Ok(()) => steps += 1,
                            Err(end) => {
                                assert_eq!(end, StepReject::EnvelopeAboveCard);
                                *count += 1;
                                break;
                            }
                        }
                    }
                }
                index += 1;
            }
            println!(
                "from rest at {} m: refused {refused_now} of 2000 steps of 1 to 4 floors, {refused_before} of 2000 at a quarter of that",
                sign * AMPLITUDE_CEILING_M
            );
            assert_eq!(refused_now, 0);
            assert!(refused_before > 0);
        }
        let (_, fresh) = scored_start();
        let mut state = fresh;
        let mut frame = blank_frame();
        step(&beam, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
        assert_eq!(state.t_s.to_bits(), floor_s.to_bits());
        assert!(state.deflection_m != fresh.deflection_m);
        assert!(state.velocity_m_s != fresh.velocity_m_s);
        assert!(state.heat_j_per_kg > 0.0);
        assert!(within(
            state.deflection_m - fresh.deflection_m,
            fresh.velocity_m_s * floor_s,
            1.0e-4
        ));
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        // A thousand more steps at the floor land and go on moving the state.
        let mut count = 0;
        while count < 1_000 {
            let before = state;
            step(&beam, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
            assert!(state.deflection_m != before.deflection_m);
            assert!(state.velocity_m_s != before.velocity_m_s);
            assert!(state.heat_j_per_kg > before.heat_j_per_kg);
            count += 1;
        }
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        // At the end of the compared run the clock still moves by a floor
        // step, to within a thousandth of it.
        let late_s = 24.2;
        let moved_s = (late_s + floor_s) - late_s;
        assert!((moved_s - floor_s).abs() <= 1.0e-3 * floor_s, "{moved_s}");

        // The step applies neither: twice the ceiling and half the floor both land.
        let mut state = fresh;
        step(&beam, &mut state, &mut frame, NO_LOAD, 2.0 * ceiling_s).unwrap();
        assert_eq!(state.t_s.to_bits(), (2.0 * ceiling_s).to_bits());
        let mut state = fresh;
        step(&beam, &mut state, &mut frame, NO_LOAD, 0.5 * floor_s).unwrap();
        assert_eq!(state.t_s.to_bits(), (0.5 * floor_s).to_bits());
        proof_intact(&frame);
    }

    #[test]
    fn halving_the_step_shows_the_slow_convergence_of_the_table() {
        // The table of `card_spec_draft.md`, section 6: the largest ledger
        // defect divided by its scale over 24 s from the start of the
        // compared run, by the card's map and its ledger with no bound applied:
        // the card does not make that start. Step, steps, the table's figure
        // (`analysis/out/05_card_emulation.log`, TEST 2).
        let table = [
            (5.0e-4, 48_000, 7.039e-7),
            (3.84e-4, 62_500, 1.950e-7),
            (2.5e-4, 96_000, 5.624e-8),
            (1.25e-4, 192_000, 1.871e-8),
        ];
        let mut worst = [0.0_f64; 4];
        let beam = CantileverEth::first_bending_mode();
        for (row, (dt_s, steps, expected)) in table.into_iter().enumerate() {
            let start = [START_DEFLECTION_M, START_VELOCITY_M_S, 0.0];
            worst[row] = ledger_on_map(&beam, &beam, start, dt_s, steps).0;
            println!("step {dt_s} s, {steps} steps: largest ledger residual {:e}", worst[row]);
            assert!(within(worst[row], expected, 1.0e-3), "step {dt_s}: {:e}", worst[row]);
            assert!(worst[row] < RESIDUAL_CONSERVATION_LIMIT);
        }
        // Fourth order would fall by 16 at each halving. From 0.5 ms to
        // 0.25 ms the ledger falls by 12.5, and from 0.25 ms to 0.125 ms by 3.0.
        let first = worst[0] / worst[2];
        let second = worst[2] / worst[3];
        println!("the ledger residual falls by {first} and then by {second} on halving the step");
        assert!(first > 12.0 && first < 13.0, "{first}");
        assert!(second > 2.9 && second < 3.1, "{second}");

        // The state after 1 s against the tight-tolerance reference of the
        // same log (DOP853, relative tolerance 1e-13), as a distance in the
        // plane of deflection and speed / ω0. Found: 1.18e-7 m at 0.5 ms,
        // 7.2e-8 m at 0.25 ms, 5.1e-8 m at 0.125 ms: 0.0012, 0.0007 and
        // 0.0005 percent of the envelope there.
        let beam = CantileverEth::first_bending_mode();
        let mut distance_m = [0.0_f64; 3];
        for (row, (dt_s, steps)) in [(5.0e-4, 2_000), (2.5e-4, 4_000), (1.25e-4, 8_000)]
            .into_iter()
            .enumerate()
        {
            let mut y = [START_DEFLECTION_M, START_VELOCITY_M_S, 0.0];
            let mut count = 0;
            while count < steps {
                y = rk4(&beam, y, dt_s).unwrap();
                count += 1;
            }
            distance_m[row] = crate::math::hypot(
                y[0] - -7.904409444440e-3,
                (y[1] - 3.130441649407e-1) / beam.omega0_rad_s(),
            );
        }
        println!(
            "distance from the tight-tolerance reference after 1 s: {:e} m, {:e} m, {:e} m",
            distance_m[0], distance_m[1], distance_m[2]
        );
        let envelope_m = beam.envelope_m(-7.904409444440e-3, 3.130441649407e-1);
        assert!(distance_m[0] < 2.0e-5 * envelope_m);
        assert!(distance_m[1] < distance_m[0]);
        assert!(distance_m[2] < distance_m[1]);
        // It falls, and by far less than 16: the corner in the damping term.
        assert!(distance_m[0] / distance_m[1] > 1.2 && distance_m[0] / distance_m[1] < 2.0);
        assert!(distance_m[1] / distance_m[2] > 1.2 && distance_m[1] / distance_m[2] < 2.0);
    }

    #[test]
    fn the_power_as_exp_of_log_is_within_its_bound_of_correctly_rounded_values() {
        let exponents = [DAMPING_POWER, SPRING_POWER - 1.0, SPRING_POWER, SPRING_POWER + 1.0];
        // The exponents the reference values were computed for.
        assert_eq!(exponents[0].to_bits(), 0x3faec26cf77a1f81);
        assert_eq!(exponents[1].to_bits(), 0x3fd108461f9f01b8);
        assert_eq!(exponents[2].to_bits(), 0x3ff4421187e7c06e);
        assert_eq!(exponents[3].to_bits(), 0x40022108c3f3e037);
        assert_eq!(POWER_IN_RANGE[0][0], 5.0e-5_f64.to_bits());
        assert_eq!(POWER_IN_RANGE[159][0], LEDGER_SCALE_AMPLITUDE_M.to_bits());
        assert_eq!(POWER_INSIDE_A_SWING[0][0], 1.0e-12_f64.to_bits());

        // The largest error of each table, in units in the last place of the
        // correctly rounded value, by exponent, and as a relative error.
        let measure = |table: &[[u64; 5]]| {
            let mut worst_units = [0_u64; 4];
            let mut worst_relative = 0.0_f64;
            for row in table {
                let size_m = f64::from_bits(row[0]);
                let mut column = 0;
                while column < 4 {
                    let got = raised(size_m, exponents[column]);
                    let expected = f64::from_bits(row[1 + column]);
                    let units = got.to_bits().abs_diff(row[1 + column]);
                    // The bound of the header: 3 |q log x| + 1 units.
                    let bound =
                        3.0 * (exponents[column] * crate::math::log(size_m)).abs() + 1.0;
                    assert!(
                        (units as f64) <= bound,
                        "{size_m:e} to the power {}: {units} units, bound {bound}",
                        exponents[column]
                    );
                    if units > worst_units[column] {
                        worst_units[column] = units;
                    }
                    let relative_error = ((got - expected) / expected).abs();
                    if relative_error > worst_relative {
                        worst_relative = relative_error;
                    }
                    column += 1;
                }
            }
            (worst_units, worst_relative)
        };

        let (units, relative_error) = measure(&POWER_IN_RANGE);
        println!(
            "power, amplitudes 5e-5 m to 0.0188 m: largest error {units:?} units in the last place for exponents s, p-1, p, p+1; relative {relative_error:e}"
        );
        // Found: 1, 3, 16 and 22 units; relative 3.6e-15.
        assert!(units[0] <= 1 && units[1] <= 3 && units[2] <= 16 && units[3] <= 22, "{units:?}");
        assert!(relative_error < 4.0e-15, "{relative_error:e}");

        let (units, relative_error) = measure(&POWER_INSIDE_A_SWING);
        println!(
            "power, deflections 1e-12 m to 5e-5 m: largest error {units:?} units in the last place; relative {relative_error:e}"
        );
        // Found: 1, 5, 23 and 55 units; relative 6.3e-15.
        assert!(units[0] <= 1 && units[1] <= 5 && units[2] <= 23 && units[3] <= 55, "{units:?}");
        assert!(relative_error < 7.0e-15, "{relative_error:e}");
    }
}
