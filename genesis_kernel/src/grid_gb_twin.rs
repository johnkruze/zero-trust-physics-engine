#![no_implicit_prelude]

//! The Great Britain synchronous area after an importing interconnector
//! trips, as a twin of the day: the inertia the operator published for the
//! half hour and the dynamic frequency response it held in that four-hour
//! block, compared over the first 60 seconds of 86 trips.
//!
//! The specimen is the 50 Hz transmission system of England, Scotland and
//! Wales as one rotating mass. Measured by the system operator (National Grid
//! ESO, since 2024 the National Energy System Operator, NESO), under the NESO
//! Open Data Licence: the system frequency once a second ("System
//! Frequency"), the half-hourly stored kinetic energy ("System Inertia",
//! "Outturn Inertia"), and the cleared volumes of its dynamic response
//! services by four-hour block ("Dynamic Containment Masterdata", "DC, DR & DM
//! Results Summary", and the Enduring Auction Capability "Response-Reserve
//! Results Summary"). The size of each loss is the interconnector's flow
//! before and after the trip in Elexon's five-minute generation by fuel type
//! (BMRS FUELINST). The trips are every single-interconnector loss of 500 to
//! 1402 MW in 114 months of frequency, May 2017 to June 2026, by the rule of
//! the `grid_gb` record applied to every hit of its scan
//! (`evidence/research/grid_gb_interconnector_loss/twin/`). No measured value
//! is stored in the card's constants or its step.
//!
//! The card is the aggregate swing equation with the kinetic energy written
//! exactly, demand's own relief with frequency, the plant's primary response
//! as one fitted function of the frequency, and NESO's three dynamic services
//! by their published curves, each at the volume held on the day:
//!
//! ```text
//! kinetic energy   E_k = H (f / 50)²                                          MW s
//! frequency        df/dt = (50² / (2 H f)) (−L + r_p + r_f + r_r + c_L D (f_s − f))
//! plant target     g(f) = K_1 (50 − f) + K_2 max(0, f_knee − f)                MW
//! plant            dr_p/dt = (g(f) − g(f_s) − r_p) / T_p
//! fast services    s_f(f) = η_DC (P_DCL φ_DC(50 − f) − P_DCH φ_DC(f − 50))
//!                         + η_DM (P_DML φ_DM(50 − f) − P_DMH φ_DM(f − 50))     MW
//!                  dr_f/dt = (s_f(f) − s_f(f_s) − r_f) / (a t_fast)
//! regulation       s_r(f) = η_DR (P_DRL φ_DR(50 − f) − P_DRH φ_DR(f − 50))    MW
//!                  dr_r/dt = (s_r(f) − s_r(f_s) − r_r) / (a t_reg)
//! ```
//!
//! `φ` are NESO's delivery curves, the fraction of the contracted volume
//! against the deviation from 50 Hz, zero inside the ±0.015 Hz deadband and
//! one past full delivery: Dynamic Containment 5 percent at 0.2 Hz and all of
//! it at 0.5 Hz; Dynamic Moderation 5 percent at 0.1 Hz and all of it at
//! 0.2 Hz; Dynamic Regulation in a straight line to all of it at 0.2 Hz
//! (NESO, Testing Guidance (DC, DM, DR), and Public Dynamic Response Services
//! Provider Guidance v14, June 2026, Table 1). The low volumes deliver below
//! 50 Hz and the high volumes absorb above it, on the same curves mirrored.
//! `t_fast` and `t_reg` are the services' times to full delivery, NESO's
//! maximum 1 s for DC and DM and 10 s for DR; each service is a first-order
//! lag of a third of that time ([`DELIVERY_LAG_PER_FULL_DELIVERY`]), so it is
//! 95 percent delivered at its time to full delivery.
//!
//! `H`, `D`, `f_s` and the six volumes and two delivery times are the area's
//! condition on the day: the caller supplies them at the start and they
//! travel in the state. Fitted on all 86 trips at once, by least squares on
//! the frequency itself, sample by sample over 60 s
//! (`twin/analysis/14_fit.py`): [`PLANT_GAIN_MW_PER_HZ`] (`K_1`),
//! [`PLANT_GAIN_BELOW_KNEE_MW_PER_HZ`] (`K_2`), [`PLANT_RESPONSE_TIME_S`]
//! (`T_p`) and two delivery shares, [`DELIVERY_SHARE_DM`] and
//! [`DELIVERY_SHARE_DR`] (`η`); Dynamic Containment delivers as contracted,
//! [`DELIVERY_SHARE_DC`] one. The knee, 49.8 Hz,
//! the deadband, the curves, the delivery times and demand's relief,
//! [`LOAD_RELIEF_PER_HZ`], 2.5 percent of demand per hertz, are published
//! figures and not fitted.
//!
//! The compared range is the span of the 86 trips: start 49.75 to 50.15 Hz,
//! inertia 111,000 to 368,000 MW s, demand 15,597 to 43,106 MW, loss 500 to
//! 1402 MW, each low volume up to the largest held (DC 1396 MW, DM 530 MW, DR
//! 480 MW), each high volume up to the largest held (DC 1637 MW, DM 590 MW, DR
//! 500 MW), the delivery times at NESO's maxima, the first 60 s and one loss
//! held from time zero. A step outside lands with
//! [`FLAG_OUTSIDE_COMPARED_TRIPS`], [`FLAG_PAST_COMPARED_MINUTE`] or
//! [`FLAG_BELOW_COMPARED_FREQUENCY`]; the equations and the ledger are the same
//! there.
//!
//! What stops the card is physical. The frequency stops it at 48.8 Hz, where
//! low-frequency demand disconnection begins
//! ([`StepReject::LowFrequencyDemandDisconnection`]), and above 50.5 Hz, the
//! statutory limit ([`StepReject::FrequencyAboveCard`]). The stored kinetic
//! energy must be above zero ([`StepReject::InertiaNotPositive`]); the demand
//! cannot be negative ([`StepReject::DemandNegative`]); the power lost cannot
//! be larger in size than the demand ([`StepReject::LossLargerThanDemand`]); a
//! contracted volume cannot be negative ([`StepReject::ServiceVolumeNegative`])
//! and a time to full delivery must be above zero, since the lag divides by it
//! ([`StepReject::DeliveryTimeNotPositive`]).
//!
//! The card leaves out the network (one frequency for the whole area),
//! generators that trip as a consequence, the operator's instructed actions
//! after the first minute, the static and slower reserves, inertia that the
//! published figure does not count (the operator's stability units and
//! grid-forming batteries), and every other synchronous area.
//!
//! The ledger is the kinetic energy of the rotating mass against the power
//! imbalance integrated in time:
//!
//! ```text
//! E_k − E_k,anchor = −∫ L dt + ∫ (r_p + r_f + r_r) dt + c_L D ∫ (f_s − f) dt
//! energy_defect_mw_s          = E_k − E_k,anchor + W_lost − W_response − W_relief
//! residual_conservation_error = |energy_defect_mw_s| / max(W_lost + |W_response| + |W_relief|, 1 MW s)
//! ```
//!
//! The step is fixed-step fourth-order Runge–Kutta on the frequency, the
//! three responses and the two integrals. It never evaluates `E_k`; the
//! ledger evaluates it from the landed frequency, so a wrong coefficient or a
//! wrong stage weight shows in the defect.
//!
//! [`GridGbTwin::gate_dt_s_at`] is the step ceiling of a run:
//! [`GATE_PHASE_STEP`] over the fastest rate the card can reach with that
//! run's condition, the largest of the three lags' inverses, the relief pole
//! `c c_L D`, and each response's swing frequency `sqrt(c S / T)` with `S`
//! its steepest slope in MW per Hz, at `c = 50² / (2 H 48.8)`.
//! [`GridGbTwin::gate_dt_s`] is the same rule at the corner of the compared
//! spans. [`GridGbTwin::floor_dt_s`] is [`FLOOR_DT_S`], 1e-6 s.
//!
//! The clock is a running sum, `t += dt`, and the comparison with the end of
//! the compared minute carries [`GridGbTwin::time_end_allowance_s`], as the
//! `grid_gb` card does.
//!
//! `core` only: no heap. The state is `Copy`. An accepted step writes time,
//! frequency, the plant's response, the services' response, rate of change of
//! frequency, the lowest landed frequency and its time, the net power and the
//! conservation residual into [`LastStateFrame64`], with the card's three
//! flags. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Nominal frequency of the area.
pub const NOMINAL_FREQUENCY_HZ: f64 = 50.0;
/// `K_1`, the plant's response per hertz below 50 Hz. Fitted on 86 trips.
pub const PLANT_GAIN_MW_PER_HZ: f64 = 2059.0;
/// `K_2`, the plant's further response per hertz below the knee. Fitted on 86 trips.
pub const PLANT_GAIN_BELOW_KNEE_MW_PER_HZ: f64 = 1987.0;
/// `T_p`, the lag of the plant's response. Fitted on 86 trips.
pub const PLANT_RESPONSE_TIME_S: f64 = 3.27;
/// The knee of the plant's response: the frequency where the operator's
/// containment passes from 5 percent of its volume towards all of it, NESO's
/// "Knee-point Frequency fn ± 0.2 Hz". Not fitted.
pub const PLANT_KNEE_HZ: f64 = 49.8;
/// Demand's own fall with frequency, per hertz, as a fraction of demand:
/// 2.5 percent per hertz (National Grid, System Operability Framework 2016,
/// p. 70). Not fitted.
pub const LOAD_RELIEF_PER_HZ: f64 = 0.025;
/// The deadband of the three dynamic services, Hz from 50 Hz: no delivery inside it.
pub const SERVICE_DEADBAND_HZ: f64 = 0.015;
/// Dynamic Containment: the knee deviation, where it delivers [`DC_KNEE_SHARE`].
pub const DC_KNEE_HZ: f64 = 0.2;
/// Dynamic Containment: the share of its volume at the knee.
pub const DC_KNEE_SHARE: f64 = 0.05;
/// Dynamic Containment: the deviation of full delivery.
pub const DC_FULL_HZ: f64 = 0.5;
/// Dynamic Moderation: the knee deviation, where it delivers [`DM_KNEE_SHARE`].
pub const DM_KNEE_HZ: f64 = 0.1;
/// Dynamic Moderation: the share of its volume at the knee.
pub const DM_KNEE_SHARE: f64 = 0.05;
/// Dynamic Moderation: the deviation of full delivery.
pub const DM_FULL_HZ: f64 = 0.2;
/// Dynamic Regulation: the deviation of full delivery, reached in a straight line from the deadband.
pub const DR_FULL_HZ: f64 = 0.2;
/// The share of each service's contracted volume the area receives: Dynamic
/// Containment, all of it, as contracted. Not fitted.
pub const DELIVERY_SHARE_DC: f64 = 1.0;
/// The share delivered: Dynamic Moderation. Fitted on 86 trips, 0.913.
pub const DELIVERY_SHARE_DM: f64 = 0.913;
/// The share delivered: Dynamic Regulation. Fitted on 86 trips: zero, the
/// smallest it may be. Within a minute its ten-second delivery is not told
/// apart from the plant's response on these trips.
pub const DELIVERY_SHARE_DR: f64 = 0.0;
/// A service's first-order lag, as a fraction of its time to full delivery:
/// a third, so a step change is 95 percent delivered at that time.
pub const DELIVERY_LAG_PER_FULL_DELIVERY: f64 = 1.0 / 3.0;
/// NESO's maximum time to full delivery of Dynamic Containment and Dynamic
/// Moderation, the time compared.
pub const FAST_DELIVERY_COMPARED_S: f64 = 1.0;
/// NESO's maximum time to full delivery of Dynamic Regulation, the time compared.
pub const REGULATION_DELIVERY_COMPARED_S: f64 = 10.0;
/// Smallest and largest starting frequency compared: the 86 trips start
/// between 49.763 and 50.126 Hz; rounded outward to 0.05 Hz.
pub const START_FREQUENCY_MIN_HZ: f64 = 49.75;
pub const START_FREQUENCY_MAX_HZ: f64 = 50.15;
/// Smallest and largest stored kinetic energy at 50 Hz compared: 111 and 368 GVA s.
pub const INERTIA_MIN_MW_S: f64 = 111_000.0;
pub const INERTIA_MAX_MW_S: f64 = 368_000.0;
/// Smallest and largest demand compared.
pub const DEMAND_MIN_MW: f64 = 15_597.0;
pub const DEMAND_MAX_MW: f64 = 43_106.0;
/// Smallest and largest loss compared.
pub const LOSS_MIN_MW: f64 = 500.0;
pub const LOSS_MAX_MW: f64 = 1402.0;
/// Largest volume held on a compared trip, each service and side, MW.
pub const DC_LOW_MAX_MW: f64 = 1396.0;
pub const DM_LOW_MAX_MW: f64 = 530.0;
pub const DR_LOW_MAX_MW: f64 = 480.0;
pub const DC_HIGH_MAX_MW: f64 = 1637.0;
pub const DM_HIGH_MAX_MW: f64 = 590.0;
pub const DR_HIGH_MAX_MW: f64 = 500.0;
/// Low-frequency demand disconnection begins here.
pub const FREQUENCY_MIN_HZ: f64 = 48.8;
/// Upper statutory limit.
pub const FREQUENCY_MAX_HZ: f64 = 50.5;
/// The lowest frequency compared with measurement: the lowest measured
/// single-loss frequency of the 86 trips, 49.553 Hz (9 May 2019), rounded
/// outward. 22 December 2023 fell to 49.275 Hz under a second loss the open
/// data does not name. A landing under it sets [`FLAG_BELOW_COMPARED_FREQUENCY`].
pub const FREQUENCY_COMPARED_MIN_HZ: f64 = 49.55;
/// The minute after the start the card was compared over.
pub const TIME_MAX_S: f64 = 60.0;
/// Half the spacing of 64-bit numbers between 32 s and 64 s, 2^-48 s.
pub const TIME_END_ALLOWANCE_PER_STEP_S: f64 = 3.552_713_678_800_501e-15;
/// Steps added to the count in [`GridGbTwin::time_end_allowance_s`], for the step read from a decimal.
pub const TIME_END_ALLOWANCE_EXTRA_STEPS: f64 = 2.0;
/// The ledger's scale is never under this: one megawatt for one second.
pub const ENERGY_SCALE_FLOOR_MW_S: f64 = 1.0;
/// `residual_conservation_error` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-7;
/// Fraction of the fastest time scale used by [`GridGbTwin::gate_dt_s`].
pub const GATE_PHASE_STEP: f64 = 0.02;
/// Smallest step the card stands behind.
pub const FLOOR_DT_S: f64 = 1.0e-6;

/// The run's start frequency, inertia, demand, a service volume or a
/// delivery time, or this step's loss, is outside the compared trips (start
/// 49.75 to 50.15 Hz, inertia 111,000 to 368,000 MW s, demand 15,597 to
/// 43,106 MW, loss 500 to 1402 MW, volumes up to the largest held, delivery
/// at 1 s and 10 s), or this step's loss is not the run's first. The step
/// still lands.
pub const FLAG_OUTSIDE_COMPARED_TRIPS: u64 = 1 << 0;
/// The step lands later than 60 s after the start, by more than the rounding
/// allowance of the clock. The step still lands.
pub const FLAG_PAST_COMPARED_MINUTE: u64 = 1 << 1;
/// The landed frequency is under 49.55 Hz, the lowest frequency compared
/// with measurement. The step still lands.
pub const FLAG_BELOW_COMPARED_FREQUENCY: u64 = 1 << 2;

/// Why a step did not advance the area or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The power lost or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A constant is not this card's.
    InvalidCard,
    /// The stored kinetic energy at 50 Hz is zero or less: the rate of change of frequency divides by it.
    InertiaNotPositive,
    /// The demand is below zero.
    DemandNegative,
    /// A contracted volume of a dynamic service is below zero.
    ServiceVolumeNegative,
    /// A time to full delivery is zero or less: the service's lag divides by it.
    DeliveryTimeNotPositive,
    /// The power lost is larger in size than the demand: the area cannot lose more infeed, or more demand, than it carries.
    LossLargerThanDemand,
    /// The clock is below zero.
    NegativeTime,
    /// The frequency, incoming or landing, is at or below 48.8 Hz, where low-frequency demand disconnection begins.
    LowFrequencyDemandDisconnection,
    /// The frequency, incoming or landing, is above 50.5 Hz, the upper statutory limit.
    FrequencyAboveCard,
    /// The trial step created kinetic energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed kinetic energy.
    FirstLawBreach,
}

/// The power lost, held through the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGbTwinLoad {
    pub loss_mw: f64,
}

/// The dynamic response held on the day: six contracted volumes and two
/// times to full delivery.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResponseHeld {
    pub dc_low_mw: f64,
    pub dm_low_mw: f64,
    pub dr_low_mw: f64,
    pub dc_high_mw: f64,
    pub dm_high_mw: f64,
    pub dr_high_mw: f64,
    pub fast_delivery_s: f64,
    pub regulation_delivery_s: f64,
}

impl ResponseHeld {
    /// No dynamic service held, delivery times at NESO's maxima: the area
    /// before October 2020.
    #[must_use]
    pub fn none() -> Self {
        Self {
            dc_low_mw: 0.0,
            dm_low_mw: 0.0,
            dr_low_mw: 0.0,
            dc_high_mw: 0.0,
            dm_high_mw: 0.0,
            dr_high_mw: 0.0,
            fast_delivery_s: FAST_DELIVERY_COMPARED_S,
            regulation_delivery_s: REGULATION_DELIVERY_COMPARED_S,
        }
    }

    fn is_finite(&self) -> bool {
        self.dc_low_mw.is_finite()
            && self.dm_low_mw.is_finite()
            && self.dr_low_mw.is_finite()
            && self.dc_high_mw.is_finite()
            && self.dm_high_mw.is_finite()
            && self.dr_high_mw.is_finite()
            && self.fast_delivery_s.is_finite()
            && self.regulation_delivery_s.is_finite()
    }

    fn volumes_not_negative(&self) -> bool {
        self.dc_low_mw >= 0.0
            && self.dm_low_mw >= 0.0
            && self.dr_low_mw >= 0.0
            && self.dc_high_mw >= 0.0
            && self.dm_high_mw >= 0.0
            && self.dr_high_mw >= 0.0
    }

    fn delivery_positive(&self) -> bool {
        self.fast_delivery_s > 0.0 && self.regulation_delivery_s > 0.0
    }
}

/// The locked card.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGbTwin {
    pub nominal_frequency_hz: f64,
    pub plant_gain_mw_per_hz: f64,
    pub plant_gain_below_knee_mw_per_hz: f64,
    pub plant_response_time_s: f64,
    pub plant_knee_hz: f64,
    pub load_relief_per_hz: f64,
    pub service_deadband_hz: f64,
    pub delivery_share_dc: f64,
    pub delivery_share_dm: f64,
    pub delivery_share_dr: f64,
    pub delivery_lag_per_full_delivery: f64,
    pub frequency_min_hz: f64,
    pub frequency_max_hz: f64,
    pub frequency_compared_min_hz: f64,
    pub time_max_s: f64,
    pub energy_scale_floor_mw_s: f64,
    pub residual_conservation_limit: f64,
    pub gate_phase_step: f64,
    pub floor_dt_s: f64,
}

impl GridGbTwin {
    /// The card.
    #[must_use]
    pub fn great_britain() -> Self {
        let card = Self {
            nominal_frequency_hz: NOMINAL_FREQUENCY_HZ,
            plant_gain_mw_per_hz: PLANT_GAIN_MW_PER_HZ,
            plant_gain_below_knee_mw_per_hz: PLANT_GAIN_BELOW_KNEE_MW_PER_HZ,
            plant_response_time_s: PLANT_RESPONSE_TIME_S,
            plant_knee_hz: PLANT_KNEE_HZ,
            load_relief_per_hz: LOAD_RELIEF_PER_HZ,
            service_deadband_hz: SERVICE_DEADBAND_HZ,
            delivery_share_dc: DELIVERY_SHARE_DC,
            delivery_share_dm: DELIVERY_SHARE_DM,
            delivery_share_dr: DELIVERY_SHARE_DR,
            delivery_lag_per_full_delivery: DELIVERY_LAG_PER_FULL_DELIVERY,
            frequency_min_hz: FREQUENCY_MIN_HZ,
            frequency_max_hz: FREQUENCY_MAX_HZ,
            frequency_compared_min_hz: FREQUENCY_COMPARED_MIN_HZ,
            time_max_s: TIME_MAX_S,
            energy_scale_floor_mw_s: ENERGY_SCALE_FLOOR_MW_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_phase_step: GATE_PHASE_STEP,
            floor_dt_s: FLOOR_DT_S,
        };
        if card.is_valid() {
            card
        } else {
            ::core::panic!("grid_gb_twin failed is_valid");
        }
    }

    /// Every copy equals its crate constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.nominal_frequency_hz.to_bits() == NOMINAL_FREQUENCY_HZ.to_bits()
            && self.plant_gain_mw_per_hz.to_bits() == PLANT_GAIN_MW_PER_HZ.to_bits()
            && self.plant_gain_below_knee_mw_per_hz.to_bits()
                == PLANT_GAIN_BELOW_KNEE_MW_PER_HZ.to_bits()
            && self.plant_response_time_s.to_bits() == PLANT_RESPONSE_TIME_S.to_bits()
            && self.plant_knee_hz.to_bits() == PLANT_KNEE_HZ.to_bits()
            && self.load_relief_per_hz.to_bits() == LOAD_RELIEF_PER_HZ.to_bits()
            && self.service_deadband_hz.to_bits() == SERVICE_DEADBAND_HZ.to_bits()
            && self.delivery_share_dc.to_bits() == DELIVERY_SHARE_DC.to_bits()
            && self.delivery_share_dm.to_bits() == DELIVERY_SHARE_DM.to_bits()
            && self.delivery_share_dr.to_bits() == DELIVERY_SHARE_DR.to_bits()
            && self.delivery_lag_per_full_delivery.to_bits()
                == DELIVERY_LAG_PER_FULL_DELIVERY.to_bits()
            && self.frequency_min_hz.to_bits() == FREQUENCY_MIN_HZ.to_bits()
            && self.frequency_max_hz.to_bits() == FREQUENCY_MAX_HZ.to_bits()
            && self.frequency_compared_min_hz.to_bits() == FREQUENCY_COMPARED_MIN_HZ.to_bits()
            && self.time_max_s.to_bits() == TIME_MAX_S.to_bits()
            && self.energy_scale_floor_mw_s.to_bits() == ENERGY_SCALE_FLOOR_MW_S.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.gate_phase_step.to_bits() == GATE_PHASE_STEP.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
    }

    /// The step ceiling at the corner of the compared spans: the smallest
    /// inertia, the largest demand and every volume at its largest, at
    /// NESO's delivery times, seconds.
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        self.gate_dt_s_at(INERTIA_MIN_MW_S, DEMAND_MAX_MW, &compared_corner())
    }

    /// The step ceiling of a run with this inertia, demand and response
    /// held, seconds: the same rule at that run's own fastest rate. `None`
    /// for an inertia not above zero, a demand below zero, a volume below
    /// zero or a delivery time not above zero.
    #[must_use]
    pub fn gate_dt_s_at(&self, inertia_mw_s: f64, demand_mw: f64, held: &ResponseHeld) -> Option<f64> {
        if !self.is_valid()
            || !(inertia_mw_s > 0.0)
            || !(demand_mw >= 0.0)
            || !held.is_finite()
            || !held.volumes_not_negative()
            || !held.delivery_positive()
        {
            return None;
        }
        let c = self.nominal_frequency_hz * self.nominal_frequency_hz
            / (2.0 * inertia_mw_s * self.frequency_min_hz);
        let plant_slope = self.plant_gain_mw_per_hz + self.plant_gain_below_knee_mw_per_hz;
        let fast_lag_s = self.delivery_lag_per_full_delivery * held.fast_delivery_s;
        let regulation_lag_s = self.delivery_lag_per_full_delivery * held.regulation_delivery_s;
        let fast_slope = self.delivery_share_dc * larger(held.dc_low_mw, held.dc_high_mw) * dc_steepest_per_hz()
            + self.delivery_share_dm * larger(held.dm_low_mw, held.dm_high_mw) * dm_steepest_per_hz();
        let regulation_slope =
            self.delivery_share_dr * larger(held.dr_low_mw, held.dr_high_mw) * dr_steepest_per_hz();
        let rates = [
            1.0 / self.plant_response_time_s,
            1.0 / fast_lag_s,
            1.0 / regulation_lag_s,
            c * self.load_relief_per_hz * demand_mw,
            (c * plant_slope / self.plant_response_time_s).sqrt(),
            (c * fast_slope / fast_lag_s).sqrt(),
            (c * regulation_slope / regulation_lag_s).sqrt(),
        ];
        let mut fastest = 0.0;
        let mut index = 0;
        while index < rates.len() {
            if rates[index] > fastest {
                fastest = rates[index];
            }
            index += 1;
        }
        if fastest.is_finite() && fastest > 0.0 {
            Some(self.gate_phase_step / fastest)
        } else {
            None
        }
    }

    /// How far past [`TIME_MAX_S`] the running sum of the clock may read at
    /// the end of a step of `dt_s`, seconds; never more than half the step.
    #[must_use]
    pub fn time_end_allowance_s(&self, dt_s: f64) -> f64 {
        if !(dt_s > 0.0) {
            return 0.0;
        }
        let steps = self.time_max_s / dt_s + TIME_END_ALLOWANCE_EXTRA_STEPS;
        let allowance_s = steps * TIME_END_ALLOWANCE_PER_STEP_S;
        if allowance_s < 0.5 * dt_s { allowance_s } else { 0.5 * dt_s }
    }

    /// The step floor, seconds.
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        if self.floor_dt_s.is_finite() && self.floor_dt_s > 0.0 {
            Some(self.floor_dt_s)
        } else {
            None
        }
    }
}

/// Every volume at the largest held on a compared trip, at NESO's delivery times.
#[must_use]
pub fn compared_corner() -> ResponseHeld {
    ResponseHeld {
        dc_low_mw: DC_LOW_MAX_MW,
        dm_low_mw: DM_LOW_MAX_MW,
        dr_low_mw: DR_LOW_MAX_MW,
        dc_high_mw: DC_HIGH_MAX_MW,
        dm_high_mw: DM_HIGH_MAX_MW,
        dr_high_mw: DR_HIGH_MAX_MW,
        fast_delivery_s: FAST_DELIVERY_COMPARED_S,
        regulation_delivery_s: REGULATION_DELIVERY_COMPARED_S,
    }
}

fn larger(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

fn dc_steepest_per_hz() -> f64 {
    let first = DC_KNEE_SHARE / (DC_KNEE_HZ - SERVICE_DEADBAND_HZ);
    let second = (1.0 - DC_KNEE_SHARE) / (DC_FULL_HZ - DC_KNEE_HZ);
    larger(first, second)
}

fn dm_steepest_per_hz() -> f64 {
    let first = DM_KNEE_SHARE / (DM_KNEE_HZ - SERVICE_DEADBAND_HZ);
    let second = (1.0 - DM_KNEE_SHARE) / (DM_FULL_HZ - DM_KNEE_HZ);
    larger(first, second)
}

fn dr_steepest_per_hz() -> f64 {
    1.0 / (DR_FULL_HZ - SERVICE_DEADBAND_HZ)
}

/// The share of a volume delivered at a deviation from 50 Hz, on a curve
/// with a deadband, a knee and full delivery: zero at or under the
/// deadband, straight lines to the knee and to full delivery, one past it.
fn curve(deviation_hz: f64, knee_hz: f64, knee_share: f64, full_hz: f64) -> f64 {
    if deviation_hz <= SERVICE_DEADBAND_HZ {
        0.0
    } else if deviation_hz <= knee_hz {
        knee_share * (deviation_hz - SERVICE_DEADBAND_HZ) / (knee_hz - SERVICE_DEADBAND_HZ)
    } else if deviation_hz < full_hz {
        knee_share + (1.0 - knee_share) * (deviation_hz - knee_hz) / (full_hz - knee_hz)
    } else {
        1.0
    }
}

fn phi_dc(deviation_hz: f64) -> f64 {
    curve(deviation_hz, DC_KNEE_HZ, DC_KNEE_SHARE, DC_FULL_HZ)
}

fn phi_dm(deviation_hz: f64) -> f64 {
    curve(deviation_hz, DM_KNEE_HZ, DM_KNEE_SHARE, DM_FULL_HZ)
}

fn phi_dr(deviation_hz: f64) -> f64 {
    if deviation_hz <= SERVICE_DEADBAND_HZ {
        0.0
    } else if deviation_hz < DR_FULL_HZ {
        (deviation_hz - SERVICE_DEADBAND_HZ) / (DR_FULL_HZ - SERVICE_DEADBAND_HZ)
    } else {
        1.0
    }
}

/// The area. `Copy`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGbTwinState {
    pub t_s: f64,
    pub frequency_hz: f64,
    /// The plant's response delivered since the start, MW.
    pub plant_response_mw: f64,
    /// Dynamic Containment and Dynamic Moderation delivered since the start, MW.
    pub fast_response_mw: f64,
    /// Dynamic Regulation delivered since the start, MW.
    pub regulation_response_mw: f64,
    /// Frequency at the start of the run, Hz. Held.
    pub start_frequency_hz: f64,
    /// Kinetic energy stored at 50 Hz, MW s. Held.
    pub inertia_mw_s: f64,
    /// Demand, MW. Held.
    pub demand_mw: f64,
    /// The dynamic response held on the day. Held.
    pub held: ResponseHeld,
    /// Demand's relief at the landed frequency, MW.
    pub relief_mw: f64,
    /// Rate of change of frequency at the landed state under the held loss, Hz per s.
    pub frequency_rate_hz_per_s: f64,
    /// −loss + responses + relief at the landed state, MW.
    pub net_power_mw: f64,
    /// Kinetic energy at the landed frequency, MW s.
    pub kinetic_energy_mw_s: f64,
    /// Kinetic energy at the start, MW s.
    pub kinetic_energy_anchor_mw_s: f64,
    /// ∫ loss dt, MW s.
    pub energy_lost_mw_s: f64,
    /// ∫ (plant + fast + regulation) dt, MW s.
    pub energy_response_mw_s: f64,
    /// ∫ (start frequency − frequency) dt, Hz s. The relief energy is this times `c_L D`.
    pub frequency_deficit_integral_hz_s: f64,
    pub energy_defect_mw_s: f64,
    pub residual_conservation_error: f64,
    /// Lowest landed frequency, and the time of it.
    pub lowest_frequency_hz: f64,
    pub lowest_time_s: f64,
    /// The loss of the run's first step, MW. Zero before the first step.
    pub run_loss_mw: f64,
}

impl GridGbTwinState {
    /// The area in balance at `start_frequency_hz`, with the day's inertia,
    /// demand and response held. `None` past a limit of the card: a start
    /// at or below 48.8 Hz or above 50.5 Hz, an inertia not above zero, a
    /// demand below zero, a volume below zero or a delivery time not above
    /// zero.
    #[must_use]
    pub fn try_start(
        card: &GridGbTwin,
        start_frequency_hz: f64,
        inertia_mw_s: f64,
        demand_mw: f64,
        held: ResponseHeld,
    ) -> Option<Self> {
        if !card.is_valid()
            || !start_frequency_hz.is_finite()
            || !inertia_mw_s.is_finite()
            || !demand_mw.is_finite()
            || !held.is_finite()
            || start_frequency_hz <= card.frequency_min_hz
            || start_frequency_hz > card.frequency_max_hz
            || !(inertia_mw_s > 0.0)
            || !(demand_mw >= 0.0)
            || !held.volumes_not_negative()
            || !held.delivery_positive()
        {
            return None;
        }
        let kinetic = kinetic_energy_mw_s(card, inertia_mw_s, start_frequency_hz);
        Some(Self {
            t_s: 0.0,
            frequency_hz: start_frequency_hz,
            plant_response_mw: 0.0,
            fast_response_mw: 0.0,
            regulation_response_mw: 0.0,
            start_frequency_hz,
            inertia_mw_s,
            demand_mw,
            held,
            relief_mw: 0.0,
            frequency_rate_hz_per_s: 0.0,
            net_power_mw: 0.0,
            kinetic_energy_mw_s: kinetic,
            kinetic_energy_anchor_mw_s: kinetic,
            energy_lost_mw_s: 0.0,
            energy_response_mw_s: 0.0,
            frequency_deficit_integral_hz_s: 0.0,
            energy_defect_mw_s: 0.0,
            residual_conservation_error: 0.0,
            lowest_frequency_hz: start_frequency_hz,
            lowest_time_s: 0.0,
            run_loss_mw: 0.0,
        })
    }

    /// The services' response delivered since the start, MW.
    #[must_use]
    pub fn service_response_mw(&self) -> f64 {
        self.fast_response_mw + self.regulation_response_mw
    }
}

/// One step. On `Err` the state and the frame are exactly as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    card: &GridGbTwin,
    state: &mut GridGbTwinState,
    frame: &mut LastStateFrame64,
    load: GridGbTwinLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(card, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("grid_gb_twin", exceedances(card, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    card: &GridGbTwin,
    state: &mut GridGbTwinState,
    frame: &mut LastStateFrame64,
    load: GridGbTwinLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(card, state, load, dt_s)?;
    let advanced = rk4(card, state, load.loss_mw, &state_vector(state), dt_s);
    commit(card, state, frame, load, dt_s, advanced)?;
    normal_or_zero_state(state);
    Ok(())
}

fn normal_or_zero_state(state: &mut GridGbTwinState) {
    state.t_s = normal_or_zero(state.t_s);
    state.frequency_hz = normal_or_zero(state.frequency_hz);
    state.plant_response_mw = normal_or_zero(state.plant_response_mw);
    state.fast_response_mw = normal_or_zero(state.fast_response_mw);
    state.regulation_response_mw = normal_or_zero(state.regulation_response_mw);
    state.start_frequency_hz = normal_or_zero(state.start_frequency_hz);
    state.inertia_mw_s = normal_or_zero(state.inertia_mw_s);
    state.demand_mw = normal_or_zero(state.demand_mw);
    state.held.dc_low_mw = normal_or_zero(state.held.dc_low_mw);
    state.held.dm_low_mw = normal_or_zero(state.held.dm_low_mw);
    state.held.dr_low_mw = normal_or_zero(state.held.dr_low_mw);
    state.held.dc_high_mw = normal_or_zero(state.held.dc_high_mw);
    state.held.dm_high_mw = normal_or_zero(state.held.dm_high_mw);
    state.held.dr_high_mw = normal_or_zero(state.held.dr_high_mw);
    state.held.fast_delivery_s = normal_or_zero(state.held.fast_delivery_s);
    state.held.regulation_delivery_s = normal_or_zero(state.held.regulation_delivery_s);
    state.relief_mw = normal_or_zero(state.relief_mw);
    state.frequency_rate_hz_per_s = normal_or_zero(state.frequency_rate_hz_per_s);
    state.net_power_mw = normal_or_zero(state.net_power_mw);
    state.kinetic_energy_mw_s = normal_or_zero(state.kinetic_energy_mw_s);
    state.kinetic_energy_anchor_mw_s = normal_or_zero(state.kinetic_energy_anchor_mw_s);
    state.energy_lost_mw_s = normal_or_zero(state.energy_lost_mw_s);
    state.energy_response_mw_s = normal_or_zero(state.energy_response_mw_s);
    state.frequency_deficit_integral_hz_s = normal_or_zero(state.frequency_deficit_integral_hz_s);
    state.energy_defect_mw_s = normal_or_zero(state.energy_defect_mw_s);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.lowest_frequency_hz = normal_or_zero(state.lowest_frequency_hz);
    state.lowest_time_s = normal_or_zero(state.lowest_time_s);
    state.run_loss_mw = normal_or_zero(state.run_loss_mw);
}

fn validate(
    card: &GridGbTwin,
    state: &GridGbTwinState,
    load: GridGbTwinLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.loss_mw.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !card.is_valid() {
        return Err(StepReject::InvalidCard);
    }
    if !(state.inertia_mw_s > 0.0) {
        return Err(StepReject::InertiaNotPositive);
    }
    if !(state.demand_mw >= 0.0) {
        return Err(StepReject::DemandNegative);
    }
    if !state.held.volumes_not_negative() {
        return Err(StepReject::ServiceVolumeNegative);
    }
    if !state.held.delivery_positive() {
        return Err(StepReject::DeliveryTimeNotPositive);
    }
    if load.loss_mw.abs() > state.demand_mw {
        return Err(StepReject::LossLargerThanDemand);
    }
    if !(state.t_s >= 0.0) {
        return Err(StepReject::NegativeTime);
    }
    if state.start_frequency_hz <= card.frequency_min_hz || state.frequency_hz <= card.frequency_min_hz {
        return Err(StepReject::LowFrequencyDemandDisconnection);
    }
    if state.start_frequency_hz > card.frequency_max_hz || state.frequency_hz > card.frequency_max_hz {
        return Err(StepReject::FrequencyAboveCard);
    }
    Ok(())
}

fn commit(
    card: &GridGbTwin,
    state: &mut GridGbTwinState,
    frame: &mut LastStateFrame64,
    load: GridGbTwinLoad,
    dt_s: f64,
    advanced: [f64; 6],
) -> Result<(), StepReject> {
    let saved = *state;
    let mut index = 0;
    while index < 6 {
        if !advanced[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    if advanced[0] <= card.frequency_min_hz {
        return Err(StepReject::LowFrequencyDemandDisconnection);
    }
    if advanced[0] > card.frequency_max_hz {
        return Err(StepReject::FrequencyAboveCard);
    }
    let flags = compared_flags(card, &saved, load, dt_s, advanced[0]);
    if state.t_s == 0.0 {
        state.run_loss_mw = load.loss_mw;
    }
    state.t_s += dt_s;
    state.frequency_hz = advanced[0];
    state.plant_response_mw = advanced[1];
    state.fast_response_mw = advanced[2];
    state.regulation_response_mw = advanced[3];
    state.energy_response_mw_s = advanced[4];
    state.frequency_deficit_integral_hz_s = advanced[5];
    state.energy_lost_mw_s += load.loss_mw * dt_s;
    let relief_gain = card.load_relief_per_hz * state.demand_mw;
    state.relief_mw = relief_gain * (state.start_frequency_hz - state.frequency_hz);
    state.net_power_mw = -load.loss_mw
        + state.plant_response_mw
        + state.fast_response_mw
        + state.regulation_response_mw
        + state.relief_mw;
    state.frequency_rate_hz_per_s = rates(card, state, load.loss_mw, &state_vector(state))[0];
    state.kinetic_energy_mw_s = kinetic_energy_mw_s(card, state.inertia_mw_s, state.frequency_hz);
    let energy_relief = relief_gain * state.frequency_deficit_integral_hz_s;
    state.energy_defect_mw_s = state.kinetic_energy_mw_s - state.kinetic_energy_anchor_mw_s
        + state.energy_lost_mw_s
        - state.energy_response_mw_s
        - energy_relief;
    let mut scale =
        state.energy_lost_mw_s.abs() + state.energy_response_mw_s.abs() + energy_relief.abs();
    if !(scale > card.energy_scale_floor_mw_s) {
        scale = card.energy_scale_floor_mw_s;
    }
    state.residual_conservation_error = state.energy_defect_mw_s.abs() / scale;
    if state.frequency_hz < state.lowest_frequency_hz {
        state.lowest_frequency_hz = state.frequency_hz;
        state.lowest_time_s = state.t_s;
    }
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > card.residual_conservation_limit {
        let created = state.energy_defect_mw_s > 0.0;
        *state = saved;
        return Err(if created {
            StepReject::SpontaneousGeneration
        } else {
            StepReject::FirstLawBreach
        });
    }
    publish(state, frame, flags);
    Ok(())
}

/// Whether the response held is inside what the compared trips held: each
/// volume from zero to the largest held, the delivery times at NESO's maxima.
fn held_inside_compared(held: &ResponseHeld) -> bool {
    held.dc_low_mw <= DC_LOW_MAX_MW
        && held.dm_low_mw <= DM_LOW_MAX_MW
        && held.dr_low_mw <= DR_LOW_MAX_MW
        && held.dc_high_mw <= DC_HIGH_MAX_MW
        && held.dm_high_mw <= DM_HIGH_MAX_MW
        && held.dr_high_mw <= DR_HIGH_MAX_MW
        && held.fast_delivery_s == FAST_DELIVERY_COMPARED_S
        && held.regulation_delivery_s == REGULATION_DELIVERY_COMPARED_S
}

fn compared_flags(
    card: &GridGbTwin,
    before: &GridGbTwinState,
    load: GridGbTwinLoad,
    dt_s: f64,
    landed_frequency_hz: f64,
) -> u64 {
    let loss_changed = before.t_s != 0.0 && load.loss_mw != before.run_loss_mw;
    let outside = before.start_frequency_hz < START_FREQUENCY_MIN_HZ
        || before.start_frequency_hz > START_FREQUENCY_MAX_HZ
        || before.inertia_mw_s < INERTIA_MIN_MW_S
        || before.inertia_mw_s > INERTIA_MAX_MW_S
        || before.demand_mw < DEMAND_MIN_MW
        || before.demand_mw > DEMAND_MAX_MW
        || load.loss_mw < LOSS_MIN_MW
        || load.loss_mw > LOSS_MAX_MW
        || !held_inside_compared(&before.held)
        || loss_changed;
    let late = !(before.t_s + dt_s <= card.time_max_s + card.time_end_allowance_s(dt_s));
    let low = landed_frequency_hz < card.frequency_compared_min_hz;
    let mut flags = 0;
    if outside {
        flags |= FLAG_OUTSIDE_COMPARED_TRIPS;
    }
    if late {
        flags |= FLAG_PAST_COMPARED_MINUTE;
    }
    if low {
        flags |= FLAG_BELOW_COMPARED_FREQUENCY;
    }
    flags
}

fn kinetic_energy_mw_s(card: &GridGbTwin, inertia_mw_s: f64, frequency_hz: f64) -> f64 {
    let ratio = frequency_hz / card.nominal_frequency_hz;
    inertia_mw_s * ratio * ratio
}

fn plant_target_mw(card: &GridGbTwin, frequency_hz: f64) -> f64 {
    let below_knee = card.plant_knee_hz - frequency_hz;
    let knee_part = if below_knee > 0.0 { below_knee } else { 0.0 };
    card.plant_gain_mw_per_hz * (card.nominal_frequency_hz - frequency_hz)
        + card.plant_gain_below_knee_mw_per_hz * knee_part
}

fn fast_target_mw(card: &GridGbTwin, held: &ResponseHeld, frequency_hz: f64) -> f64 {
    let below = card.nominal_frequency_hz - frequency_hz;
    let above = frequency_hz - card.nominal_frequency_hz;
    card.delivery_share_dc * (held.dc_low_mw * phi_dc(below) - held.dc_high_mw * phi_dc(above))
        + card.delivery_share_dm * (held.dm_low_mw * phi_dm(below) - held.dm_high_mw * phi_dm(above))
}

fn regulation_target_mw(card: &GridGbTwin, held: &ResponseHeld, frequency_hz: f64) -> f64 {
    let below = card.nominal_frequency_hz - frequency_hz;
    let above = frequency_hz - card.nominal_frequency_hz;
    card.delivery_share_dr * (held.dr_low_mw * phi_dr(below) - held.dr_high_mw * phi_dr(above))
}

fn state_vector(state: &GridGbTwinState) -> [f64; 6] {
    [
        state.frequency_hz,
        state.plant_response_mw,
        state.fast_response_mw,
        state.regulation_response_mw,
        state.energy_response_mw_s,
        state.frequency_deficit_integral_hz_s,
    ]
}

/// Rates of frequency, the three responses, ∫ response and ∫ (start − frequency).
fn rates(card: &GridGbTwin, state: &GridGbTwinState, loss_mw: f64, y: &[f64; 6]) -> [f64; 6] {
    let held = &state.held;
    let start = state.start_frequency_hz;
    let deficit_hz = start - y[0];
    let relief_mw = card.load_relief_per_hz * state.demand_mw * deficit_hz;
    let response_mw = y[1] + y[2] + y[3];
    let net_mw = -loss_mw + response_mw + relief_mw;
    let plant = plant_target_mw(card, y[0]) - plant_target_mw(card, start);
    let fast = fast_target_mw(card, held, y[0]) - fast_target_mw(card, held, start);
    let regulation = regulation_target_mw(card, held, y[0]) - regulation_target_mw(card, held, start);
    let fast_lag_s = card.delivery_lag_per_full_delivery * held.fast_delivery_s;
    let regulation_lag_s = card.delivery_lag_per_full_delivery * held.regulation_delivery_s;
    [
        card.nominal_frequency_hz * card.nominal_frequency_hz / (2.0 * state.inertia_mw_s * y[0])
            * net_mw,
        (plant - y[1]) / card.plant_response_time_s,
        (fast - y[2]) / fast_lag_s,
        (regulation - y[3]) / regulation_lag_s,
        response_mw,
        deficit_hz,
    ]
}

fn advance(y: &[f64; 6], k: &[f64; 6], scale: f64) -> [f64; 6] {
    let mut out = [0.0; 6];
    let mut index = 0;
    while index < 6 {
        out[index] = y[index] + scale * k[index];
        index += 1;
    }
    out
}

fn rk4(card: &GridGbTwin, state: &GridGbTwinState, loss_mw: f64, y: &[f64; 6], dt_s: f64) -> [f64; 6] {
    let k1 = rates(card, state, loss_mw, y);
    let k2 = rates(card, state, loss_mw, &advance(y, &k1, 0.5 * dt_s));
    let k3 = rates(card, state, loss_mw, &advance(y, &k2, 0.5 * dt_s));
    let k4 = rates(card, state, loss_mw, &advance(y, &k3, dt_s));
    let mut out = [0.0; 6];
    let mut index = 0;
    while index < 6 {
        out[index] =
            y[index] + dt_s / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    out
}

fn publish(state: &GridGbTwinState, frame: &mut LastStateFrame64, flags: u64) {
    frame.t = state.t_s;
    frame.pos[0] = state.frequency_hz as f32;
    frame.pos[1] = state.plant_response_mw as f32;
    frame.pos[2] = state.service_response_mw() as f32;
    frame.vel[0] = state.frequency_rate_hz_per_s as f32;
    frame.vel[1] = state.lowest_frequency_hz as f32;
    frame.vel[2] = state.lowest_time_s as f32;
    frame.force_torque = state.net_power_mw as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = flags;
}

fn state_is_finite(state: &GridGbTwinState) -> bool {
    state.t_s.is_finite()
        && state.frequency_hz.is_finite()
        && state.plant_response_mw.is_finite()
        && state.fast_response_mw.is_finite()
        && state.regulation_response_mw.is_finite()
        && state.start_frequency_hz.is_finite()
        && state.inertia_mw_s.is_finite()
        && state.demand_mw.is_finite()
        && state.held.is_finite()
        && state.relief_mw.is_finite()
        && state.frequency_rate_hz_per_s.is_finite()
        && state.net_power_mw.is_finite()
        && state.kinetic_energy_mw_s.is_finite()
        && state.kinetic_energy_anchor_mw_s.is_finite()
        && state.energy_lost_mw_s.is_finite()
        && state.energy_response_mw_s.is_finite()
        && state.frequency_deficit_integral_hz_s.is_finite()
        && state.energy_defect_mw_s.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.lowest_frequency_hz.is_finite()
        && state.lowest_time_s.is_finite()
        && state.run_loss_mw.is_finite()
}


/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    TRIPS_START_LOW,
    TRIPS_START_HIGH,
    TRIPS_INERTIA_LOW,
    TRIPS_INERTIA_HIGH,
    TRIPS_DEMAND_LOW,
    TRIPS_DEMAND_HIGH,
    TRIPS_LOSS_LOW,
    TRIPS_LOSS_HIGH,
    TRIPS_DC_LOW,
    TRIPS_DM_LOW,
    TRIPS_DR_LOW,
    TRIPS_DC_HIGH,
    TRIPS_DM_HIGH,
    TRIPS_DR_HIGH,
    TRIPS_FAST_DELIVERY,
    TRIPS_REGULATION_DELIVERY,
    TRIPS_LOSS_CHANGED,
    PAST_MINUTE,
    BELOW_FREQUENCY,
];

use crate::exceedance::{Exceedance, ExceedanceKind, Exceedances, Side};

const fn trips(quantity: &'static str, unit: &'static str, side: Side, limit: f64, words: &'static str) -> ExceedanceKind {
    ExceedanceKind::fixed("FLAG_OUTSIDE_COMPARED_TRIPS", FLAG_OUTSIDE_COMPARED_TRIPS, quantity, unit, side, limit, words)
}
const TRIPS_WORDS_START: &str = "the span of start frequencies of the compared trips, 49.75 to 50.15 Hz";
const TRIPS_WORDS_INERTIA: &str = "the span of inertia of the compared trips, 111,000 to 368,000 MW s";
const TRIPS_WORDS_DEMAND: &str = "the span of demand of the compared trips, 15,597 to 43,106 MW";
const TRIPS_WORDS_LOSS: &str = "the span of losses of the compared trips, 500 to 1402 MW";
const TRIPS_START_LOW: ExceedanceKind = trips("start_frequency_hz", "Hz", Side::Below, START_FREQUENCY_MIN_HZ, TRIPS_WORDS_START);
const TRIPS_START_HIGH: ExceedanceKind = trips("start_frequency_hz", "Hz", Side::Above, START_FREQUENCY_MAX_HZ, TRIPS_WORDS_START);
const TRIPS_INERTIA_LOW: ExceedanceKind = trips("inertia_mw_s", "MW s", Side::Below, INERTIA_MIN_MW_S, TRIPS_WORDS_INERTIA);
const TRIPS_INERTIA_HIGH: ExceedanceKind = trips("inertia_mw_s", "MW s", Side::Above, INERTIA_MAX_MW_S, TRIPS_WORDS_INERTIA);
const TRIPS_DEMAND_LOW: ExceedanceKind = trips("demand_mw", "MW", Side::Below, DEMAND_MIN_MW, TRIPS_WORDS_DEMAND);
const TRIPS_DEMAND_HIGH: ExceedanceKind = trips("demand_mw", "MW", Side::Above, DEMAND_MAX_MW, TRIPS_WORDS_DEMAND);
const TRIPS_LOSS_LOW: ExceedanceKind = trips("loss_mw", "MW", Side::Below, LOSS_MIN_MW, TRIPS_WORDS_LOSS);
const TRIPS_LOSS_HIGH: ExceedanceKind = trips("loss_mw", "MW", Side::Above, LOSS_MAX_MW, TRIPS_WORDS_LOSS);
const TRIPS_DC_LOW: ExceedanceKind =
    trips("dc_low_mw", "MW", Side::Above, DC_LOW_MAX_MW, "the largest Dynamic Containment low-frequency volume held on a compared trip, 1396 MW");
const TRIPS_DM_LOW: ExceedanceKind =
    trips("dm_low_mw", "MW", Side::Above, DM_LOW_MAX_MW, "the largest Dynamic Moderation low-frequency volume held on a compared trip, 530 MW");
const TRIPS_DR_LOW: ExceedanceKind =
    trips("dr_low_mw", "MW", Side::Above, DR_LOW_MAX_MW, "the largest Dynamic Regulation low-frequency volume held on a compared trip, 480 MW");
const TRIPS_DC_HIGH: ExceedanceKind =
    trips("dc_high_mw", "MW", Side::Above, DC_HIGH_MAX_MW, "the largest Dynamic Containment high-frequency volume held on a compared trip, 1637 MW");
const TRIPS_DM_HIGH: ExceedanceKind =
    trips("dm_high_mw", "MW", Side::Above, DM_HIGH_MAX_MW, "the largest Dynamic Moderation high-frequency volume held on a compared trip, 590 MW");
const TRIPS_DR_HIGH: ExceedanceKind =
    trips("dr_high_mw", "MW", Side::Above, DR_HIGH_MAX_MW, "the largest Dynamic Regulation high-frequency volume held on a compared trip, 500 MW");
const TRIPS_FAST_DELIVERY: ExceedanceKind = trips(
    "fast_delivery_s",
    "s",
    Side::OtherThan,
    FAST_DELIVERY_COMPARED_S,
    "NESO's maximum time to full delivery of Dynamic Containment and Moderation, the time compared, 1 s",
);
const TRIPS_REGULATION_DELIVERY: ExceedanceKind = trips(
    "regulation_delivery_s",
    "s",
    Side::OtherThan,
    REGULATION_DELIVERY_COMPARED_S,
    "NESO's maximum time to full delivery of Dynamic Regulation, the time compared, 10 s",
);
const TRIPS_LOSS_CHANGED: ExceedanceKind = ExceedanceKind::varying(
    "FLAG_OUTSIDE_COMPARED_TRIPS",
    FLAG_OUTSIDE_COMPARED_TRIPS,
    "loss_mw",
    "MW",
    Side::OtherThan,
    "the run's first loss: each compared trip held one loss",
);
const PAST_MINUTE: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_PAST_COMPARED_MINUTE",
    FLAG_PAST_COMPARED_MINUTE,
    "t_s",
    "s",
    Side::Above,
    TIME_MAX_S,
    "the minute after the trip compared with measurement, 60 s; set past it by more than the clock's rounding allowance",
);
const BELOW_FREQUENCY: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_BELOW_COMPARED_FREQUENCY",
    FLAG_BELOW_COMPARED_FREQUENCY,
    "frequency_hz",
    "Hz",
    Side::Below,
    FREQUENCY_COMPARED_MIN_HZ,
    "the lowest frequency compared with measurement, 49.55 Hz",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks. The
/// conditions are those of [`compared_flags`] and [`held_inside_compared`],
/// each with its own number.
pub fn exceedances(
    card: &GridGbTwin,
    before: &GridGbTwinState,
    after: &GridGbTwinState,
    load: GridGbTwinLoad,
    dt_s: f64,
) -> Exceedances {
    let mut records = Exceedances::new();
    let start = before.start_frequency_hz;
    records.push_if(start < START_FREQUENCY_MIN_HZ, Exceedance::fixed(TRIPS_START_LOW, start));
    records.push_if(start > START_FREQUENCY_MAX_HZ, Exceedance::fixed(TRIPS_START_HIGH, start));
    let inertia = before.inertia_mw_s;
    records.push_if(inertia < INERTIA_MIN_MW_S, Exceedance::fixed(TRIPS_INERTIA_LOW, inertia));
    records.push_if(inertia > INERTIA_MAX_MW_S, Exceedance::fixed(TRIPS_INERTIA_HIGH, inertia));
    let demand = before.demand_mw;
    records.push_if(demand < DEMAND_MIN_MW, Exceedance::fixed(TRIPS_DEMAND_LOW, demand));
    records.push_if(demand > DEMAND_MAX_MW, Exceedance::fixed(TRIPS_DEMAND_HIGH, demand));
    let loss = load.loss_mw;
    records.push_if(loss < LOSS_MIN_MW, Exceedance::fixed(TRIPS_LOSS_LOW, loss));
    records.push_if(loss > LOSS_MAX_MW, Exceedance::fixed(TRIPS_LOSS_HIGH, loss));
    let held = &before.held;
    records.push_if(!(held.dc_low_mw <= DC_LOW_MAX_MW), Exceedance::fixed(TRIPS_DC_LOW, held.dc_low_mw));
    records.push_if(!(held.dm_low_mw <= DM_LOW_MAX_MW), Exceedance::fixed(TRIPS_DM_LOW, held.dm_low_mw));
    records.push_if(!(held.dr_low_mw <= DR_LOW_MAX_MW), Exceedance::fixed(TRIPS_DR_LOW, held.dr_low_mw));
    records.push_if(!(held.dc_high_mw <= DC_HIGH_MAX_MW), Exceedance::fixed(TRIPS_DC_HIGH, held.dc_high_mw));
    records.push_if(!(held.dm_high_mw <= DM_HIGH_MAX_MW), Exceedance::fixed(TRIPS_DM_HIGH, held.dm_high_mw));
    records.push_if(!(held.dr_high_mw <= DR_HIGH_MAX_MW), Exceedance::fixed(TRIPS_DR_HIGH, held.dr_high_mw));
    records.push_if(
        !(held.fast_delivery_s == FAST_DELIVERY_COMPARED_S),
        Exceedance::fixed(TRIPS_FAST_DELIVERY, held.fast_delivery_s),
    );
    records.push_if(
        !(held.regulation_delivery_s == REGULATION_DELIVERY_COMPARED_S),
        Exceedance::fixed(TRIPS_REGULATION_DELIVERY, held.regulation_delivery_s),
    );
    records.push_if(
        before.t_s != 0.0 && loss != before.run_loss_mw,
        Exceedance::at(TRIPS_LOSS_CHANGED, before.run_loss_mw, loss),
    );
    let landed_s = before.t_s + dt_s;
    records.push_if(
        !(landed_s <= card.time_max_s + card.time_end_allowance_s(dt_s)),
        Exceedance::at(PAST_MINUTE, card.time_max_s, landed_s),
    );
    records.push_if(
        after.frequency_hz < card.frequency_compared_min_hz,
        Exceedance::at(BELOW_FREQUENCY, card.frequency_compared_min_hz, after.frequency_hz),
    );
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::*;

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

    /// 17 November 2025, 06:19:28 UTC: IFA, 1007 MW, 177 GVA s, 26,774 MW,
    /// start 49.948 Hz; held DC 1145/1080, DM 500/510, DR 480/480 MW. Inputs only.
    fn nov_2025() -> (f64, f64, f64, ResponseHeld, f64) {
        let held = ResponseHeld {
            dc_low_mw: 1145.0,
            dm_low_mw: 500.0,
            dr_low_mw: 480.0,
            dc_high_mw: 1080.0,
            dm_high_mw: 510.0,
            dr_high_mw: 480.0,
            fast_delivery_s: 1.0,
            regulation_delivery_s: 10.0,
        };
        (49.948, 177_000.0, 26_774.0, held, 1007.0)
    }

    fn run(f0: f64, h: f64, d: f64, held: ResponseHeld, loss: f64, dt: f64, steps: usize) -> (GridGbTwinState, f64, u64) {
        let card = GridGbTwin::great_britain();
        let mut state = GridGbTwinState::try_start(&card, f0, h, d, held).unwrap();
        let mut frame = blank_frame();
        let mut worst = 0.0;
        let mut flags = 0;
        for _ in 0..steps {
            step(&card, &mut state, &mut frame, GridGbTwinLoad { loss_mw: loss }, dt).unwrap();
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
            flags |= frame.flags;
        }
        (state, worst, flags)
    }

    #[test]
    fn constants_are_locked() {
        let card = GridGbTwin::great_britain();
        assert!(card.is_valid());
        let mut altered = card;
        altered.delivery_share_dm = 0.9;
        assert!(!altered.is_valid());
        let (f0, h, d, held, _) = nov_2025();
        let mut state = GridGbTwinState::try_start(&card, f0, h, d, held).unwrap();
        let mut frame = blank_frame();
        assert_eq!(
            step(&altered, &mut state, &mut frame, GridGbTwinLoad { loss_mw: 1000.0 }, 0.01),
            Err(StepReject::InvalidCard)
        );
    }

    #[test]
    fn the_service_curves_are_neso_s() {
        assert_eq!(phi_dc(0.015), 0.0);
        assert!((phi_dc(0.2) - 0.05).abs() < 1e-15);
        assert_eq!(phi_dc(0.5), 1.0);
        assert!((phi_dm(0.1) - 0.05).abs() < 1e-15);
        assert_eq!(phi_dm(0.2), 1.0);
        assert!((phi_dr(0.1075) - 0.5).abs() < 1e-12);
        assert_eq!(phi_dr(0.2), 1.0);
        assert_eq!(phi_dr(-0.3), 0.0);
    }

    /// With no loss the start is in balance and stays there, whatever is held.
    #[test]
    fn the_start_is_in_balance() {
        let (f0, h, d, held, _) = nov_2025();
        let (state, worst, _) = run(f0, h, d, held, 0.0, 0.01, 500);
        assert_eq!(state.frequency_hz, f0);
        assert_eq!(worst, 0.0);
        let (state, _, _) = run(50.11, h, d, held, 0.0, 0.01, 500);
        assert_eq!(state.frequency_hz, 50.11);
    }

    /// A minute of the November 2025 trip under the ceiling: the ledger holds,
    /// no flag, and the fall is arrested. The held response makes it
    /// shallower than with none held.
    #[test]
    fn a_minute_of_a_trip_holds_its_ledger_and_the_services_arrest_it() {
        let (f0, h, d, held, loss) = nov_2025();
        let card = GridGbTwin::great_britain();
        let gate = card.gate_dt_s_at(h, d, &held).unwrap();
        let n = (60.0 / gate) as usize + 1;
        let dt = 60.0 / n as f64;
        let (with, worst, flags) = run(f0, h, d, held, loss, dt, n);
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst}");
        assert_eq!(flags, 0);
        let (without, _, _) = run(f0, h, d, ResponseHeld::none(), loss, dt, n);
        assert!(with.lowest_frequency_hz > without.lowest_frequency_hz);
        assert!(with.lowest_frequency_hz > 49.7 && with.lowest_frequency_hz < f0);
        assert!(with.service_response_mw() > 0.0);
    }

    #[test]
    fn named_limits_refuse_and_nothing_moves() {
        let card = GridGbTwin::great_britain();
        let (f0, h, d, held, _) = nov_2025();
        let mut bad = held;
        bad.dm_low_mw = -1.0;
        assert!(GridGbTwinState::try_start(&card, f0, h, d, bad).is_none());
        let mut bad = held;
        bad.fast_delivery_s = 0.0;
        assert!(GridGbTwinState::try_start(&card, f0, h, d, bad).is_none());
        let mut state = GridGbTwinState::try_start(&card, f0, h, d, held).unwrap();
        state.held.regulation_delivery_s = 0.0;
        let before = state;
        let mut frame = blank_frame();
        assert_eq!(step(&card, &mut state, &mut frame, GridGbTwinLoad { loss_mw: 1000.0 }, 0.01), Err(StepReject::DeliveryTimeNotPositive));
        assert_eq!(state, before);
        state.held.regulation_delivery_s = 10.0;
        state.held.dc_high_mw = -5.0;
        assert_eq!(step(&card, &mut state, &mut frame, GridGbTwinLoad { loss_mw: 1000.0 }, 0.01), Err(StepReject::ServiceVolumeNegative));
        state.held.dc_high_mw = 0.0;
        assert_eq!(step(&card, &mut state, &mut frame, GridGbTwinLoad { loss_mw: 30_000.0 }, 0.01), Err(StepReject::LossLargerThanDemand));
        assert_eq!(frame.flags, 9);
    }

    /// A delivery time other than NESO's maxima, a volume above the largest
    /// held, or a loss outside 500 to 1402 MW lands with the flag.
    #[test]
    fn outside_the_compared_trips_lands_with_the_flag() {
        let (f0, h, d, held, loss) = nov_2025();
        let mut faster = held;
        faster.fast_delivery_s = 0.5;
        let (_, _, flags) = run(f0, h, d, faster, loss, 0.01, 10);
        assert_eq!(flags & FLAG_OUTSIDE_COMPARED_TRIPS, FLAG_OUTSIDE_COMPARED_TRIPS);
        let mut more = held;
        more.dm_low_mw = 531.0;
        let (_, _, flags) = run(f0, h, d, more, loss, 0.01, 10);
        assert_eq!(flags & FLAG_OUTSIDE_COMPARED_TRIPS, FLAG_OUTSIDE_COMPARED_TRIPS);
        let (_, _, flags) = run(f0, h, d, held, 499.0, 0.01, 10);
        assert_eq!(flags & FLAG_OUTSIDE_COMPARED_TRIPS, FLAG_OUTSIDE_COMPARED_TRIPS);
        let (_, _, flags) = run(f0, h, d, held, 500.0, 0.01, 10);
        assert_eq!(flags, 0);
    }

    /// A ceiling halved twice converges: the lowest frequency of a minute at
    /// the ceiling and at a quarter of it agree to 1e-6 Hz.
    #[test]
    fn the_ceiling_converges() {
        let (f0, h, d, held, loss) = nov_2025();
        let card = GridGbTwin::great_britain();
        let gate = card.gate_dt_s_at(h, d, &held).unwrap();
        let n = (60.0 / gate) as usize + 1;
        let (a, _, _) = run(f0, h, d, held, loss, 60.0 / n as f64, n);
        let (b, _, _) = run(f0, h, d, held, loss, 15.0 / n as f64, 4 * n);
        assert!((a.lowest_frequency_hz - b.lowest_frequency_hz).abs() < 1e-6);
    }
}
