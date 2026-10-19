#![no_implicit_prelude]

//! One commercial microwave link in Gothenburg, Sweden, in rain: link 10010,
//! sublink 587 (direction A) of the OpenMRG data set. 32.445 GHz, vertical
//! polarisation, 1.25841 km, a 0.3 m antenna at each end, sent level 5 dBm.
//!
//! Measured by Andersson, Olsson, van de Beek, Hansryd, Andersson and
//! Persson, The OpenMRG data set, Zenodo, doi: 10.5281/zenodo.7107689,
//! CC BY-SA 4.0: the sent and the received level every 10 s from June to
//! August 2015, and the weighing rain gauge at Barlastplatsen, 37 m from the
//! path, every minute.
//!
//! The card gives the path loss of the link, in decibels, from the rain rate
//! at the gauge. Two things take power out of the beam: rain along the path,
//! and water on the two antenna covers.
//!
//! The rain term is Recommendation ITU-R P.838-3 at this frequency and
//! polarisation over this length. Nothing in it is fitted. The water on the
//! antennas has three constants, the most it adds and the times it takes to
//! build and to dry. All three are fitted to this link's own readings in the
//! 18 rain events of June and July 2015. The dry level of the link is not a
//! constant of the card: the caller supplies it as the starting state, from
//! the link's readings before the rain.
//!
//! Left out: rain on the path that differs from rain at the gauge, which
//! stands at one end; drift of the dry level during an event; dew; snow and
//! sleet; the temperature and the drop sizes of the rain. A rain rate below
//! zero is [`StepReject::NegativeRainRate`]. Any rate from zero up is
//! computed: P.838-3 is a recommendation for any rain rate, the water on the
//! antennas builds toward its own ceiling at any rate above zero, and the map
//! and the power ledger are exact at any rate. The constants were fitted on
//! one-minute rates of 0 or 6 to 84 mm/h, and compared with a month whose
//! rain reached 54 mm/h; no rate above 84 mm/h was measured at this gauge.
//!
//! The gauge counts 0.1 mm, so each of its one-minute rates is 0 or at least
//! 6 mm/h, and every rate the card was fitted and compared on is one of
//! those. A rate above zero and below 6 mm/h is computed as any other: the
//! rain term is `k R^α L` at that rate, and the water on the antennas builds
//! toward 2.13 dB as at any rate above zero, which was fitted only at rates of
//! 6 mm/h and up. The fit and the comparison fed rain held through whole gauge
//! minutes at a step of 60 s; the map of the water on the antennas is exact
//! for any rate held over a step, at any step length, so the rain may change
//! at any step. The state still carries the seconds spent in the current
//! gauge minute and the rate of the last step, as reported quantities: no
//! limit depends on them. A minute is complete when its clock is within
//! [`GAUGE_MINUTE_ALLOWANCE_S`] of 60 s, half the step floor.
//!
//! The dry level is the caller's. Below [`DRY_PATH_LOSS_FLOOR_DB`], 44.32 dB,
//! the free-space loss of this path, 124.667 dB, less the gain of two ideal
//! circular apertures of 0.3 m at this frequency, 40.172 dB each, it is
//! [`StepReject::DryPathLossBelowFreeSpace`]: no pair of these antennas has
//! that much gain. The 18 fitted events lay between 51.75 and 52.55 dB and
//! the held-back month between 52.60 and 52.90 dB.
//!
//! Every reading of August 2015 was held back from the fit. The protocol,
//! fixed before August was opened, and the fit are in
//! `evidence/research/link_openmrg_gothenburg/`. The card was compared with
//! the nine rain events of August on 2026-10-09 and met the three lines fixed
//! in advance: root mean square error 0.581 dB against 0.90, 0.585 of the
//! error of P.838 alone against 0.80, and 1.069 of the measured summed
//! attenuation, inside 0.75 to 1.25. In two of the nine events P.838 alone
//! was closer than the card. August's rain reached 54 mm/h. The record is
//! `RESULT.md` in that folder.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, path loss, rain attenuation, the attenuation
//! of the water on the antennas, the received level, the received power, the
//! received energy, the rain-rate margin, and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.
//!
//! The rain rate is held through the step. The public step is the exact map
//! of the antenna water, not a Runge–Kutta step:
//!
//! ```text
//! rain on the path          A = k R^α L                 dB    k, α: P.838-3; L in km
//! water on the antennas     dW/dt = (W_max − W) / τ_w   while R > 0
//!                           dW/dt = −W / τ_d            while R = 0
//!
//! x  = 1 − exp(−dt/τ)
//! W' = W_max − ((W_max − W) − (W_max − W) x)   while R > 0
//! W' = W − W x                                 while R = 0
//!
//! path loss                 dry level + A + W'          dB
//! received level            sent level − path loss      dBm
//! power from a level        P = 1 mW · 10^(level / 10)
//! ```
//!
//! The ledger is power, kept as energy over the run. The power that would
//! arrive over the dry path is the power received, plus the power the rain
//! takes out, plus the power the water on the antennas takes out:
//!
//! ```text
//! c = ln 10 / 10
//! P_dry      = 1 mW · exp(c (sent level − dry level))
//! P_received = P_dry exp(−c (A + W'))
//! P_rain     = P_dry (1 − exp(−c A))
//! P_antennas = P_dry exp(−c A) (1 − exp(−c W'))
//!
//! residual_conservation_error = |E_received + E_rain + E_antennas − E_dry| / E_dry
//! ```
//!
//! Each energy is its power integrated exactly over the step, with the water
//! on the antennas following its exponential through the step. With
//! `u = exp(−t/τ)`, `W(t) = W_max − (W_max − W) u` while it rains and
//! `W(t) = W u` while it is dry, and
//!
//! ```text
//! S(a) = Σ_{n≥1} a^n (1 − exp(−n dt/τ)) / (n n!)
//! ∫ exp(−c W(t)) dt = exp(−c W_max) (dt + τ S(c (W_max − W)))   while R > 0
//!                   = dt + τ S(−c W)                            while R = 0
//! E_received = P_dry exp(−c A) ∫ exp(−c W(t)) dt
//! E_antennas = P_dry exp(−c A) ∫ (1 − exp(−c W(t))) dt
//! E_rain     = P_dry (1 − exp(−c A)) dt
//! ```
//!
//! `|a|` is at most `c W_max`, 0.4904, and the series is summed to its
//! 24th term, past which a term is under 1e-32 of `dt`. One step and many
//! steps over the same span give the same energies to rounding. The ledger
//! fixes how the dry-path energy divides. It does not fix the attenuation
//! itself, which is the model, and it does not fix how the energy that
//! is not the rain's divides between the receiver and the antennas: that
//! division is the integral above.
//!
//! The card also keeps one account of the fitted model. It is bookkeeping
//! that follows from the model and not a conservation law:
//!
//! ```text
//! I = ∫ W dt over the step     W_max dt − (W_max − W) τ_w x   while R > 0
//!                              W τ_d x                         while R = 0
//! built = (W_max dt − I) / τ_w   while R > 0
//! dried = I / τ_d                while R = 0
//! antenna_account_error = |W − W_anchor − Σ built + Σ dried| / W_max
//! ```
//!
//! An antenna attenuation that is not the exact map breaks this account and
//! is [`StepReject::AntennaAccountBreach`].
//!
//! [`LinkGothenburg10010::gate_dt_s`] is set by the arithmetic of the step
//! and not by the data: the map is exact at any step, and the one number that
//! loses figures as the step grows is the water built in the step, `W_max dt −
//! I`, a difference of two numbers near `W_max dt`. Its rounding, one part in
//! 2^52 of `W_max dt / τ_w` at most, is held at a thousandth of the antenna
//! account's limit: [`GATE_DT_S`], `ANTENNA_ACCOUNT_LIMIT / 1000 × τ_w ×
//! 2^52`, 364 791 569 s, about 11.6 years, in one step. The three constants
//! were fitted at the gauge's 60 s step; one step and many steps over the same
//! span give the same state and the same energies to rounding.
//! [`LinkGothenburg10010::floor_dt_s`], about 5.7e-5 s, the gauge's minute
//! over 2^20, is the smallest step the card stands behind. The public step
//! applies neither. The gateway refuses a step outside the two.
//!
//! The step calls [`crate::math::exp`], [`crate::math::exp_m1`] and
//! [`crate::math::log`].

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Carrier frequency of sublink 587. The dataset, `cml/cml_metadata.csv`.
pub const FREQUENCY_GHZ: f64 = 32.445;
/// Distance between the two antennas. The dataset, `cml/cml_metadata.csv`.
pub const PATH_LENGTH_KM: f64 = 1.25841;
/// Diameter of the antenna at each end. The dataset, `cml/cml_metadata.csv`.
/// Enters no step; used by [`ideal_aperture_gain_db`].
pub const ANTENNA_DIAMETER_M: f64 = 0.3;
/// Sent level. The dataset, `tsl` of sublink 587: 5.0 dBm in every minute of
/// June and July 2015.
pub const SENT_LEVEL_DBM: f64 = 5.0;
/// Speed of light in vacuum, exact by definition of the metre.
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;
/// Rain coefficient `k` at 32.445 GHz, vertical polarisation, in dB/km at
/// 1 mm/h. Recommendation ITU-R P.838-3, equation (2) with Table 2. Not fitted.
pub const RAIN_COEFFICIENT_DB_PER_KM: f64 = 0.272_827_142_869_215_33;
/// Rain exponent `α` at 32.445 GHz, vertical polarisation. Recommendation
/// ITU-R P.838-3, equation (3) with Table 4. Not fitted.
pub const RAIN_EXPONENT: f64 = 0.894_780_599_816_734_9;
/// Most the water on the two antennas adds to the path loss. Fitted, this
/// link, the 18 rain events of June and July 2015.
pub const ANTENNA_WATER_MAX_DB: f64 = 2.13;
/// Time the water on the antennas takes to build while it rains. Fitted,
/// this link, the 18 rain events of June and July 2015.
pub const ANTENNA_WETTING_TIME_S: f64 = 81.0;
/// Time the water on the antennas takes to dry once the rain stops. Fitted,
/// this link, the 18 rain events of June and July 2015.
pub const ANTENNA_DRYING_TIME_S: f64 = 404.0;
/// Highest one-minute rain rate at the gauge in the fitting data. A rate above
/// it is computed; the frame's margin to it is then below zero.
pub const RAIN_RATE_MAX_MM_H: f64 = 84.0;
/// Lowest dry level a pair of these antennas can have: the free-space loss
/// over this path at this frequency, [`free_space_loss_db`], less the gain of
/// two ideal circular apertures of the antenna's diameter,
/// [`ideal_aperture_gain_db`], `124.667 − 2 × 40.172` dB. An aperture of a
/// given area has no more gain than the ideal one, so a lower dry level is a
/// gain no pair of 0.3 m dishes can have. The two functions, with the crate's
/// own logarithm, give 44.3232538878642 dB; this is that number rounded
/// down at the eighth decimal, so the bound never refuses a level they allow.
pub const DRY_PATH_LOSS_FLOOR_DB: f64 = 44.32325388;
/// The gauge's minute, and the step the three fitted constants were fitted at.
pub const GAUGE_STEP_S: f64 = 60.0;
/// The gauge's minute divided by the step floor: 2^20.
pub const GAUGE_STEP_OVER_FLOOR: f64 = 1_048_576.0;
/// Largest step the card stands behind, s: `ANTENNA_ACCOUNT_LIMIT / 1000 ×
/// ANTENNA_WETTING_TIME_S × 2^52`. The water built in a wet step is `(W_max dt
/// − I) / τ_w`, a difference of two numbers near `W_max dt`, and it rounds by
/// up to one part in 2^52 of `W_max dt / τ_w`; at this step that is a
/// thousandth of the antenna account's limit. Nothing else in the step loses
/// figures as the step grows: the water's map, its integral and each energy
/// are exact at any step.
pub const GATE_DT_S: f64 = 364_791_569.817_010_2;
/// A gauge minute is complete when its clock is within this of 60 s: half the
/// step floor, `GAUGE_STEP_S / 2^21`, about 2.9e-5 s. A sum of steps that
/// should make a whole minute rounds by far less, and no step the gateway
/// takes is shorter than twice this.
pub const GAUGE_MINUTE_ALLOWANCE_S: f64 = GAUGE_STEP_S / (2.0 * GAUGE_STEP_OVER_FLOOR);
/// Terms of the series that integrates each energy over a step.
const ENERGY_SERIES_TERMS: u32 = 24;
/// `ln 10 / 10`: a decibel as a natural logarithm of a power ratio.
pub const NEPER_PER_DB: f64 = core::f64::consts::LN_10 / 10.0;
/// One milliwatt, the reference of the dBm.
pub const MILLIWATT_W: f64 = 1.0e-3;

/// `|E_received + E_rain + E_antennas − E_dry| / E_dry` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;
/// `|W − W_anchor − Σ built + Σ dried| / W_max` must stay at or under this.
/// At the step floor each step rounds the water by up to one part in 10^15
/// of its ceiling, so a million such steps stay a thousand times under it.
pub const ANTENNA_ACCOUNT_LIMIT: f64 = 1.0e-6;

/// It rained through the step that wrote this frame. The step still lands.
pub const FLAG_RAINING: u64 = 1 << 0;

/// Why a step did not advance the link or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The rain rate or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this link's.
    InvalidLink,
    /// The rain rate is below zero: rain does not fall upward.
    NegativeRainRate,
    /// The seconds already spent in the current gauge minute are below zero or not below 60 s.
    GaugeMinuteOutsideCard,
    /// The dry level is below 44.32 dB, the free-space loss of this path less the gain of two ideal 0.3 m apertures: no pair of these antennas has that much gain.
    DryPathLossBelowFreeSpace,
    /// The attenuation of the water on the antennas is below zero or above the most the card gives it, 2.13 dB.
    AntennaWaterOutsideCard,
    /// The trial step delivered more power than the dry path carries.
    SpontaneousGeneration,
    /// The trial step lost power that neither the rain nor the water on the antennas took.
    FirstLawBreach,
    /// The trial step broke the fitted model's account of the water on the antennas, which is bookkeeping that follows from the model and not a conservation law.
    AntennaAccountBreach,
}

/// The rain at the gauge, held through the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkLoad {
    /// Rain rate at the Barlastplatsen gauge, mm/h. Zero or more.
    pub rain_rate_mm_h: f64,
}

/// The locked link. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkGothenburg10010 {
    /// Recorded copy of [`FREQUENCY_GHZ`].
    pub frequency_ghz: f64,
    /// Recorded copy of [`PATH_LENGTH_KM`].
    pub path_length_km: f64,
    /// Recorded copy of [`ANTENNA_DIAMETER_M`].
    pub antenna_diameter_m: f64,
    /// Recorded copy of [`SENT_LEVEL_DBM`].
    pub sent_level_dbm: f64,
    /// Recorded copy of [`RAIN_COEFFICIENT_DB_PER_KM`].
    pub rain_coefficient_db_per_km: f64,
    /// Recorded copy of [`RAIN_EXPONENT`].
    pub rain_exponent: f64,
    /// Recorded copy of [`ANTENNA_WATER_MAX_DB`].
    pub antenna_water_max_db: f64,
    /// Recorded copy of [`ANTENNA_WETTING_TIME_S`].
    pub antenna_wetting_time_s: f64,
    /// Recorded copy of [`ANTENNA_DRYING_TIME_S`].
    pub antenna_drying_time_s: f64,
    /// Recorded copy of [`RAIN_RATE_MAX_MM_H`].
    pub rain_rate_max_mm_h: f64,
    /// Recorded copy of [`DRY_PATH_LOSS_FLOOR_DB`].
    pub dry_path_loss_floor_db: f64,
    /// Recorded copy of [`GAUGE_STEP_S`].
    pub gauge_step_s: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`ANTENNA_ACCOUNT_LIMIT`].
    pub antenna_account_limit: f64,
}

impl LinkGothenburg10010 {
    /// Card: OpenMRG link 10010, sublink 587. The constructor takes no arguments.
    #[must_use]
    pub fn sublink_587() -> Self {
        let link = Self {
            frequency_ghz: FREQUENCY_GHZ,
            path_length_km: PATH_LENGTH_KM,
            antenna_diameter_m: ANTENNA_DIAMETER_M,
            sent_level_dbm: SENT_LEVEL_DBM,
            rain_coefficient_db_per_km: RAIN_COEFFICIENT_DB_PER_KM,
            rain_exponent: RAIN_EXPONENT,
            antenna_water_max_db: ANTENNA_WATER_MAX_DB,
            antenna_wetting_time_s: ANTENNA_WETTING_TIME_S,
            antenna_drying_time_s: ANTENNA_DRYING_TIME_S,
            rain_rate_max_mm_h: RAIN_RATE_MAX_MM_H,
            dry_path_loss_floor_db: DRY_PATH_LOSS_FLOOR_DB,
            gauge_step_s: GAUGE_STEP_S,
            gate_dt_s: GATE_DT_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            antenna_account_limit: ANTENNA_ACCOUNT_LIMIT,
        };
        if link.is_valid() {
            link
        } else {
            ::core::panic!("Gothenburg link 10010 failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.frequency_ghz.to_bits() == FREQUENCY_GHZ.to_bits()
            && self.path_length_km.to_bits() == PATH_LENGTH_KM.to_bits()
            && self.antenna_diameter_m.to_bits() == ANTENNA_DIAMETER_M.to_bits()
            && self.sent_level_dbm.to_bits() == SENT_LEVEL_DBM.to_bits()
            && self.rain_coefficient_db_per_km.to_bits() == RAIN_COEFFICIENT_DB_PER_KM.to_bits()
            && self.rain_exponent.to_bits() == RAIN_EXPONENT.to_bits()
            && self.antenna_water_max_db.to_bits() == ANTENNA_WATER_MAX_DB.to_bits()
            && self.antenna_wetting_time_s.to_bits() == ANTENNA_WETTING_TIME_S.to_bits()
            && self.antenna_drying_time_s.to_bits() == ANTENNA_DRYING_TIME_S.to_bits()
            && self.rain_rate_max_mm_h.to_bits() == RAIN_RATE_MAX_MM_H.to_bits()
            && self.dry_path_loss_floor_db.to_bits() == DRY_PATH_LOSS_FLOOR_DB.to_bits()
            && self.gauge_step_s.to_bits() == GAUGE_STEP_S.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.antenna_account_limit.to_bits() == ANTENNA_ACCOUNT_LIMIT.to_bits()
            && self.antenna_water_max_db > 0.0
            && self.antenna_wetting_time_s > 0.0
            && self.antenna_drying_time_s > 0.0
            && self.rain_rate_max_mm_h > 0.0
            && self.dry_path_loss_floor_db > 0.0
            && self.gate_dt_s > self.gauge_step_s
    }

    /// Largest step the card stands behind: [`GATE_DT_S`], set by the
    /// rounding of the water built in one step, a thousandth of the antenna
    /// account's limit. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let gate = self.gate_dt_s;
        if gate.is_finite() && gate > 0.0 {
            Some(gate)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: the gauge's minute divided by
    /// 2^20, about 5.7e-5 s. At that step the water on the antennas moves by
    /// at most seven parts in ten million of its ceiling, which 64-bit
    /// arithmetic still resolves to nine figures. Under that a step is mostly
    /// rounding. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        let gauge = self.gauge_step_s;
        if gauge.is_finite() && gauge > 0.0 {
            Some(gauge / GAUGE_STEP_OVER_FLOOR)
        } else {
            None
        }
    }

    /// Attenuation of rain along the path, `k R^α L`, in dB. Zero at no rain.
    #[must_use]
    pub fn rain_attenuation_db(&self, rain_rate_mm_h: f64) -> f64 {
        if rain_rate_mm_h > 0.0 {
            self.rain_coefficient_db_per_km
                * crate::math::exp(self.rain_exponent * crate::math::log(rain_rate_mm_h))
                * self.path_length_km
        } else {
            0.0
        }
    }

    /// Power of a level: `1 mW · 10^(level / 10)`, in watts.
    #[must_use]
    pub fn power_w_from_level_dbm(&self, level_dbm: f64) -> f64 {
        MILLIWATT_W * crate::math::exp(NEPER_PER_DB * level_dbm)
    }
}

/// Loss between two isotropic antennas in free space over this path at this
/// frequency, `20 log10(4 π d f / c)`: 124.667 dB. A reference; it enters no
/// step.
#[must_use]
pub fn free_space_loss_db() -> f64 {
    let ratio = 4.0 * core::f64::consts::PI * (PATH_LENGTH_KM * 1.0e3) * (FREQUENCY_GHZ * 1.0e9)
        / SPEED_OF_LIGHT_M_S;
    2.0 * crate::math::log(ratio) / NEPER_PER_DB
}

/// Gain of an ideal circular aperture of the antenna's diameter at this
/// frequency, `20 log10(π D f / c)`: 40.172 dB. A reference; it enters no
/// step. The dataset gives no gain of the real antennas.
#[must_use]
pub fn ideal_aperture_gain_db() -> f64 {
    let ratio =
        core::f64::consts::PI * ANTENNA_DIAMETER_M * (FREQUENCY_GHZ * 1.0e9) / SPEED_OF_LIGHT_M_S;
    2.0 * crate::math::log(ratio) / NEPER_PER_DB
}

/// Integrated link. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkState {
    /// Time since the start of the run.
    pub t_s: f64,
    /// Sent level minus received level with no rain and dry antennas. The caller's.
    pub dry_path_loss_db: f64,
    /// Attenuation of the water on the two antennas, `W`.
    pub antenna_water_db: f64,
    /// Rain rate of the last step.
    pub rain_rate_mm_h: f64,
    /// `k R^α L` of the last step.
    pub rain_attenuation_db: f64,
    /// `dry_path_loss_db + rain_attenuation_db + antenna_water_db`.
    pub path_loss_db: f64,
    /// `SENT_LEVEL_DBM − path_loss_db`.
    pub received_level_dbm: f64,
    /// Power of the received level.
    pub received_power_w: f64,
    /// Power that would arrive over the dry path.
    pub dry_path_power_w: f64,
    /// Energy the dry path would have delivered since the anchor.
    pub dry_path_energy_j: f64,
    /// Energy received since the anchor.
    pub received_energy_j: f64,
    /// Energy the rain took out since the anchor.
    pub rain_energy_j: f64,
    /// Energy the water on the antennas took out since the anchor.
    pub antenna_energy_j: f64,
    /// Signed `E_received + E_rain + E_antennas − E_dry`. Positive is created energy.
    pub energy_defect_j: f64,
    /// `|energy_defect_j| / dry_path_energy_j`. The energy ledger alone.
    pub residual_conservation_error: f64,
    /// `W` at the state the account started from.
    pub antenna_account_anchor_db: f64,
    /// Attenuation built on the antennas since the anchor, from the integral of `W`.
    pub antenna_built_db: f64,
    /// Attenuation dried off the antennas since the anchor, from the integral of `W`.
    pub antenna_dried_db: f64,
    /// Signed `W − W_anchor − built + dried`.
    pub antenna_account_defect_db: f64,
    /// `|antenna_account_defect_db| / W_max`. An account of the fitted model:
    /// bookkeeping that follows from the model and not a conservation law.
    pub antenna_account_error: f64,
    /// `RAIN_RATE_MAX_MM_H − rain_rate_mm_h`: below zero for a rate above the
    /// highest of the fitting data.
    pub rain_rate_margin_mm_h: f64,
    /// Seconds already spent in the current gauge minute, from 0 to under 60.
    /// Zero on a minute's boundary. Reported; no limit depends on it.
    pub gauge_minute_elapsed_s: f64,
    /// Rain rate of the last step, or the rate stated at the start. Reported;
    /// no limit depends on it.
    pub gauge_minute_rain_mm_h: f64,
    /// It rained through the last step.
    pub raining: bool,
}

impl LinkState {
    /// Anchor both ledgers at a dry level and an attenuation of the water on
    /// the antennas, on a gauge minute's boundary. A link that has been dry
    /// for an hour starts at zero.
    #[must_use]
    pub fn try_new(
        link: &LinkGothenburg10010,
        dry_path_loss_db: f64,
        antenna_water_db: f64,
    ) -> Option<Self> {
        Self::try_continue(link, dry_path_loss_db, antenna_water_db, 0.0, 0.0).ok()
    }

    /// Anchor both ledgers as [`Self::try_new`] does, part way through a
    /// gauge minute: `gauge_minute_elapsed_s` seconds of it already spent,
    /// with `gauge_minute_rain_mm_h` the rate of the last step. Both are
    /// reported and no limit depends on them; the rate must not be below zero.
    pub fn try_continue(
        link: &LinkGothenburg10010,
        dry_path_loss_db: f64,
        antenna_water_db: f64,
        gauge_minute_elapsed_s: f64,
        gauge_minute_rain_mm_h: f64,
    ) -> Result<Self, StepReject> {
        if !dry_path_loss_db.is_finite()
            || !antenna_water_db.is_finite()
            || !gauge_minute_elapsed_s.is_finite()
            || !gauge_minute_rain_mm_h.is_finite()
        {
            return Err(StepReject::NonFiniteState);
        }
        if !link.is_valid() {
            return Err(StepReject::InvalidLink);
        }
        if !dry_level_possible(dry_path_loss_db) {
            return Err(StepReject::DryPathLossBelowFreeSpace);
        }
        if !antenna_water_on_card(antenna_water_db) {
            return Err(StepReject::AntennaWaterOutsideCard);
        }
        if !gauge_minute_on_card(gauge_minute_elapsed_s) {
            return Err(StepReject::GaugeMinuteOutsideCard);
        }
        rain_rate_check(gauge_minute_rain_mm_h)?;
        let path_loss_db = dry_path_loss_db + antenna_water_db;
        let received_level_dbm = link.sent_level_dbm - path_loss_db;
        let state = Self {
            t_s: 0.0,
            dry_path_loss_db,
            antenna_water_db,
            rain_rate_mm_h: 0.0,
            rain_attenuation_db: 0.0,
            path_loss_db,
            received_level_dbm,
            received_power_w: link.power_w_from_level_dbm(received_level_dbm),
            dry_path_power_w: link.power_w_from_level_dbm(link.sent_level_dbm - dry_path_loss_db),
            dry_path_energy_j: 0.0,
            received_energy_j: 0.0,
            rain_energy_j: 0.0,
            antenna_energy_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            antenna_account_anchor_db: antenna_water_db,
            antenna_built_db: 0.0,
            antenna_dried_db: 0.0,
            antenna_account_defect_db: 0.0,
            antenna_account_error: 0.0,
            rain_rate_margin_mm_h: link.rain_rate_max_mm_h,
            // Negative zero is zero.
            gauge_minute_elapsed_s: gauge_minute_elapsed_s + 0.0,
            gauge_minute_rain_mm_h: gauge_minute_rain_mm_h + 0.0,
            raining: false,
        };
        if state_is_finite(&state) {
            Ok(state)
        } else {
            Err(StepReject::NonFiniteState)
        }
    }
}

/// Advance one exact step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    link: &LinkGothenburg10010,
    state: &mut LinkState,
    frame: &mut LastStateFrame64,
    load: LinkLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(link, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("link_gothenburg_10010", exceedances(link, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    link: &LinkGothenburg10010,
    state: &mut LinkState,
    frame: &mut LastStateFrame64,
    load: LinkLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(link, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    link: &LinkGothenburg10010,
    state: &mut LinkState,
    frame: &mut LastStateFrame64,
    load: LinkLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(link, state, load, dt_s)?;
    let advanced = match exact_map(link, state, load, dt_s) {
        Some(stepped) => stepped,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(link, state, frame, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut LinkState) {
    state.t_s = normal_or_zero(state.t_s);
    state.dry_path_loss_db = normal_or_zero(state.dry_path_loss_db);
    state.antenna_water_db = normal_or_zero(state.antenna_water_db);
    state.rain_rate_mm_h = normal_or_zero(state.rain_rate_mm_h);
    state.rain_attenuation_db = normal_or_zero(state.rain_attenuation_db);
    state.path_loss_db = normal_or_zero(state.path_loss_db);
    state.received_level_dbm = normal_or_zero(state.received_level_dbm);
    state.received_power_w = normal_or_zero(state.received_power_w);
    state.dry_path_power_w = normal_or_zero(state.dry_path_power_w);
    state.dry_path_energy_j = normal_or_zero(state.dry_path_energy_j);
    state.received_energy_j = normal_or_zero(state.received_energy_j);
    state.rain_energy_j = normal_or_zero(state.rain_energy_j);
    state.antenna_energy_j = normal_or_zero(state.antenna_energy_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.antenna_account_anchor_db = normal_or_zero(state.antenna_account_anchor_db);
    state.antenna_built_db = normal_or_zero(state.antenna_built_db);
    state.antenna_dried_db = normal_or_zero(state.antenna_dried_db);
    state.antenna_account_defect_db = normal_or_zero(state.antenna_account_defect_db);
    state.antenna_account_error = normal_or_zero(state.antenna_account_error);
    state.rain_rate_margin_mm_h = normal_or_zero(state.rain_rate_margin_mm_h);
    state.gauge_minute_elapsed_s = normal_or_zero(state.gauge_minute_elapsed_s);
    state.gauge_minute_rain_mm_h = normal_or_zero(state.gauge_minute_rain_mm_h);
}

/// A rain rate of zero or more.
fn rain_rate_check(rain_rate_mm_h: f64) -> Result<(), StepReject> {
    if rain_rate_mm_h >= 0.0 {
        Ok(())
    } else {
        Err(StepReject::NegativeRainRate)
    }
}

fn gauge_minute_on_card(gauge_minute_elapsed_s: f64) -> bool {
    gauge_minute_elapsed_s >= 0.0 && gauge_minute_elapsed_s < GAUGE_STEP_S
}

/// The clock of the gauge minute after `dt_s` more seconds: the remainder
/// after whole minutes, set to zero within [`GAUGE_MINUTE_ALLOWANCE_S`] of a
/// boundary on either side.
fn gauge_minute_after(gauge_minute_elapsed_s: f64, dt_s: f64) -> f64 {
    let into = (gauge_minute_elapsed_s + dt_s) % GAUGE_STEP_S;
    if into < GAUGE_MINUTE_ALLOWANCE_S || into > GAUGE_STEP_S - GAUGE_MINUTE_ALLOWANCE_S {
        0.0
    } else {
        into
    }
}

fn dry_level_possible(dry_path_loss_db: f64) -> bool {
    dry_path_loss_db >= DRY_PATH_LOSS_FLOOR_DB
}

fn antenna_water_on_card(antenna_water_db: f64) -> bool {
    antenna_water_db >= 0.0 && antenna_water_db <= ANTENNA_WATER_MAX_DB
}

fn validate(
    link: &LinkGothenburg10010,
    state: &LinkState,
    load: LinkLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.rain_rate_mm_h.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !link.is_valid() {
        return Err(StepReject::InvalidLink);
    }
    rain_rate_check(load.rain_rate_mm_h)?;
    if !dry_level_possible(state.dry_path_loss_db) {
        return Err(StepReject::DryPathLossBelowFreeSpace);
    }
    if !antenna_water_on_card(state.antenna_water_db) {
        return Err(StepReject::AntennaWaterOutsideCard);
    }
    if !gauge_minute_on_card(state.gauge_minute_elapsed_s) {
        return Err(StepReject::GaugeMinuteOutsideCard);
    }
    rain_rate_check(state.gauge_minute_rain_mm_h)

}

struct ExactStep {
    /// `W` at the end of the step.
    antenna_water_db: f64,
    /// `∫ W dt` over the step, dB s.
    antenna_water_integral_db_s: f64,
    /// `k R^α L`.
    rain_attenuation_db: f64,
    /// Power received at the end of the step.
    received_power_w: f64,
    /// Energy received over the step.
    received_energy_j: f64,
    /// Energy the rain takes out over the step.
    rain_energy_j: f64,
    /// Energy the water on the antennas takes out over the step.
    antenna_energy_j: f64,
}

/// `S(a) = Σ_{n≥1} a^n (1 − exp(−n dt/τ)) / (n n!)`, to its 24th term.
fn water_series(a: f64, dt_s: f64, time_s: f64) -> f64 {
    let mut sum = 0.0;
    // a^n / n!
    let mut rising = 1.0;
    let mut n = 1_u32;
    while n <= ENERGY_SERIES_TERMS {
        let order = n as f64;
        rising *= a / order;
        let shed = -crate::math::exp_m1(-order * dt_s / time_s);
        sum += rising * shed / order;
        n += 1;
    }
    sum
}

/// The antenna water from its exponential and its integral from the closed
/// form; the end-of-step power received from its expression; each energy
/// from its power integrated exactly over the step.
fn exact_map(
    link: &LinkGothenburg10010,
    state: &LinkState,
    load: LinkLoad,
    dt_s: f64,
) -> Option<ExactStep> {
    let water_max_db = link.antenna_water_max_db;
    let water_db = state.antenna_water_db;
    let raining = load.rain_rate_mm_h > 0.0;
    let time_s = if raining {
        link.antenna_wetting_time_s
    } else {
        link.antenna_drying_time_s
    };
    if !(time_s > 0.0) {
        return None;
    }
    // `1 − exp(−dt/τ)`, between 0 and 1. What is short of the ceiling, or
    // what is on the antennas, loses this share, so neither passes its bound.
    let shed = -crate::math::exp_m1(-dt_s / time_s);
    let (next_water_db, integral_db_s) = if raining {
        let short_db = water_max_db - water_db;
        (
            water_max_db - (short_db - short_db * shed),
            water_max_db * dt_s - short_db * time_s * shed,
        )
    } else {
        (water_db - water_db * shed, water_db * time_s * shed)
    };
    let rain_attenuation_db = link.rain_attenuation_db(load.rain_rate_mm_h);
    let dry_power_w = state.dry_path_power_w;
    let received_power_w =
        dry_power_w * crate::math::exp(-NEPER_PER_DB * (rain_attenuation_db + next_water_db));
    // `∫ exp(−c W(t)) dt` and `∫ (1 − exp(−c W(t))) dt` over the step; the
    // two sum to `dt`.
    let (passed_s, held_s) = if raining {
        let series = water_series(NEPER_PER_DB * (water_max_db - water_db), dt_s, time_s);
        let at_ceiling = crate::math::exp(-NEPER_PER_DB * water_max_db);
        (
            at_ceiling * (dt_s + time_s * series),
            dt_s * -crate::math::exp_m1(-NEPER_PER_DB * water_max_db)
                - at_ceiling * time_s * series,
        )
    } else {
        let series = water_series(-NEPER_PER_DB * water_db, dt_s, time_s);
        (dt_s + time_s * series, -(time_s * series))
    };
    let past_rain = crate::math::exp(-NEPER_PER_DB * rain_attenuation_db);
    let received_energy_j = dry_power_w * past_rain * passed_s;
    let antenna_energy_j = dry_power_w * past_rain * held_s;
    let rain_energy_j =
        dry_power_w * -crate::math::exp_m1(-NEPER_PER_DB * rain_attenuation_db) * dt_s;
    if next_water_db.is_finite()
        && integral_db_s.is_finite()
        && rain_attenuation_db.is_finite()
        && received_power_w.is_finite()
        && received_energy_j.is_finite()
        && rain_energy_j.is_finite()
        && antenna_energy_j.is_finite()
    {
        Some(ExactStep {
            antenna_water_db: next_water_db,
            antenna_water_integral_db_s: integral_db_s,
            rain_attenuation_db,
            received_power_w,
            received_energy_j,
            rain_energy_j,
            antenna_energy_j,
        })
    } else {
        None
    }
}

fn commit(
    link: &LinkGothenburg10010,
    state: &mut LinkState,
    frame: &mut LastStateFrame64,
    load: LinkLoad,
    dt_s: f64,
    advanced: ExactStep,
) -> Result<(), StepReject> {
    let saved = *state;
    if !advanced.antenna_water_db.is_finite()
        || !advanced.antenna_water_integral_db_s.is_finite()
        || !advanced.rain_attenuation_db.is_finite()
        || !advanced.received_power_w.is_finite()
        || !advanced.received_energy_j.is_finite()
        || !advanced.rain_energy_j.is_finite()
        || !advanced.antenna_energy_j.is_finite()
    {
        return Err(StepReject::NonFiniteState);
    }
    let raining = load.rain_rate_mm_h > 0.0;
    state.antenna_water_db = advanced.antenna_water_db;
    state.rain_rate_mm_h = load.rain_rate_mm_h;
    state.rain_attenuation_db = advanced.rain_attenuation_db;
    state.path_loss_db =
        state.dry_path_loss_db + advanced.rain_attenuation_db + advanced.antenna_water_db;
    state.received_level_dbm = link.sent_level_dbm - state.path_loss_db;
    state.received_power_w = advanced.received_power_w;
    state.dry_path_energy_j += state.dry_path_power_w * dt_s;
    state.received_energy_j += advanced.received_energy_j;
    state.rain_energy_j += advanced.rain_energy_j;
    state.antenna_energy_j += advanced.antenna_energy_j;
    if raining {
        state.antenna_built_db += (link.antenna_water_max_db * dt_s
            - advanced.antenna_water_integral_db_s)
            / link.antenna_wetting_time_s;
    } else {
        state.antenna_dried_db +=
            advanced.antenna_water_integral_db_s / link.antenna_drying_time_s;
    }
    state.t_s += dt_s;
    state.rain_rate_margin_mm_h = link.rain_rate_max_mm_h - load.rain_rate_mm_h;
    state.gauge_minute_elapsed_s = gauge_minute_after(state.gauge_minute_elapsed_s, dt_s);
    state.gauge_minute_rain_mm_h = load.rain_rate_mm_h + 0.0;
    state.raining = raining;
    let scale = state.dry_path_energy_j;
    if !(scale > 0.0 && scale.is_finite()) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    state.energy_defect_j = state.received_energy_j + state.rain_energy_j
        + state.antenna_energy_j
        - state.dry_path_energy_j;
    state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    state.antenna_account_defect_db = state.antenna_water_db
        - state.antenna_account_anchor_db
        - state.antenna_built_db
        + state.antenna_dried_db;
    state.antenna_account_error =
        state.antenna_account_defect_db.abs() / link.antenna_water_max_db;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    let gate = RESIDUAL_CONSERVATION_LIMIT * scale;
    if state.energy_defect_j > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    if state.antenna_account_error > ANTENNA_ACCOUNT_LIMIT {
        *state = saved;
        return Err(StepReject::AntennaAccountBreach);
    }
    if !antenna_water_on_card(state.antenna_water_db) {
        *state = saved;
        return Err(StepReject::AntennaWaterOutsideCard);
    }
    publish(state, frame);
    Ok(())
}

fn publish(state: &LinkState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.path_loss_db as f32;
    frame.pos[1] = state.rain_attenuation_db as f32;
    frame.pos[2] = state.antenna_water_db as f32;
    frame.vel[0] = state.received_level_dbm as f32;
    frame.vel[1] = state.received_power_w as f32;
    frame.vel[2] = state.received_energy_j as f32;
    frame.force_torque = state.rain_rate_margin_mm_h as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.raining { FLAG_RAINING } else { 0 };
}

fn state_is_finite(state: &LinkState) -> bool {
    state.t_s.is_finite()
        && state.dry_path_loss_db.is_finite()
        && state.antenna_water_db.is_finite()
        && state.rain_rate_mm_h.is_finite()
        && state.rain_attenuation_db.is_finite()
        && state.path_loss_db.is_finite()
        && state.received_level_dbm.is_finite()
        && state.received_power_w.is_finite()
        && state.dry_path_power_w.is_finite()
        && state.dry_path_energy_j.is_finite()
        && state.received_energy_j.is_finite()
        && state.rain_energy_j.is_finite()
        && state.antenna_energy_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.antenna_account_anchor_db.is_finite()
        && state.antenna_built_db.is_finite()
        && state.antenna_dried_db.is_finite()
        && state.antenna_account_defect_db.is_finite()
        && state.antenna_account_error.is_finite()
        && state.rain_rate_margin_mm_h.is_finite()
        && state.gauge_minute_elapsed_s.is_finite()
        && state.gauge_minute_rain_mm_h.is_finite()
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    RAIN_KIND,
];

const RAIN_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_RAINING",
    FLAG_RAINING,
    "rain_rate_mm_h",
    "mm/h",
    Side::Above,
    0.0,
    "no rain: a rain rate of zero; the extent is the step's rain rate",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
pub fn exceedances(
    link: &LinkGothenburg10010,
    before: &LinkState,
    after: &LinkState,
    load: LinkLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (link, before, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.raining, Exceedance::fixed(RAIN_KIND, load.rain_rate_mm_h));
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        ANTENNA_ACCOUNT_LIMIT, ANTENNA_DRYING_TIME_S, ANTENNA_WATER_MAX_DB, ANTENNA_WETTING_TIME_S,
        DRY_PATH_LOSS_FLOOR_DB, ExactStep, FLAG_RAINING, FREQUENCY_GHZ, GATE_DT_S,
        LinkGothenburg10010, LinkLoad, LinkState, NEPER_PER_DB, PATH_LENGTH_KM,
        RAIN_COEFFICIENT_DB_PER_KM, RAIN_EXPONENT, RAIN_RATE_MAX_MM_H,
        RESIDUAL_CONSERVATION_LIMIT, SENT_LEVEL_DBM, StepReject, commit, exact_map,
        free_space_loss_db, ideal_aperture_gain_db, step, validate,
    };
    use crate::frame::LastStateFrame64;

    const MINUTE_S: f64 = 60.0;
    const DRY_DB: f64 = 52.3;

    fn zero_frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: 0.0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            force_torque: 0.0,
            residual: 0.0,
            flags: 0,
            proof: [0xA5; 16],
        }
    }

    fn rain(rain_rate_mm_h: f64) -> LinkLoad {
        LinkLoad { rain_rate_mm_h }
    }

    fn dry_start() -> (LinkGothenburg10010, LinkState, LastStateFrame64) {
        let link = LinkGothenburg10010::sublink_587();
        let state = LinkState::try_new(&link, DRY_DB, 0.0).expect("a dry start");
        (link, state, zero_frame())
    }

    /// A refusal leaves the state and the frame, proof included, as they were.
    fn refused(
        link: &LinkGothenburg10010,
        state: &LinkState,
        load: LinkLoad,
        dt_s: f64,
        expected: StepReject,
    ) {
        let mut trial = *state;
        let mut frame = zero_frame();
        let before = frame.to_bytes();
        assert_eq!(step(link, &mut trial, &mut frame, load, dt_s), Err(expected));
        // Compared field by field through Debug so a NaN planted in the state still matches itself.
        assert_eq!(std::format!("{trial:?}"), std::format!("{state:?}"));
        assert_eq!(frame.to_bytes(), before);
    }

    /// P.838-3 equations (2) and (3) with Tables 2 and 4, computed here with
    /// the platform's functions, give the two recorded coefficients.
    #[test]
    fn the_rain_coefficients_are_recommendation_p838_at_this_frequency() {
        let log_f = FREQUENCY_GHZ.log10();
        let bell = |a: f64, b: f64, c: f64| a * (-((log_f - b) / c).powi(2)).exp();
        let log_k = bell(-3.80595, 0.56934, 0.81061)
            + bell(-3.44965, -0.22911, 0.51059)
            + bell(-0.39902, 0.73042, 0.11899)
            + bell(0.50167, 1.07319, 0.27195)
            + -0.16398 * log_f
            + 0.63297;
        let alpha = bell(-0.07771, 2.33840, -0.76284)
            + bell(0.56727, 0.95545, 0.54039)
            + bell(-0.20238, 1.14520, 0.26809)
            + bell(-48.2991, 0.791669, 0.116226)
            + bell(48.5833, 0.791459, 0.116479)
            + -0.053739 * log_f
            + 0.83433;
        assert!((10.0_f64.powf(log_k) - RAIN_COEFFICIENT_DB_PER_KM).abs() < 1.0e-13);
        assert!((alpha - RAIN_EXPONENT).abs() < 1.0e-12);
    }

    /// The recommendation's own Table 5 at 30 and 35 GHz, vertical, brackets
    /// the card's coefficients: k 0.2291 and 0.3224, α 0.9129 and 0.8761.
    #[test]
    fn the_rain_coefficients_lie_between_the_tabulated_neighbours() {
        assert!(RAIN_COEFFICIENT_DB_PER_KM > 0.2291 && RAIN_COEFFICIENT_DB_PER_KM < 0.3224);
        assert!(RAIN_EXPONENT < 0.9129 && RAIN_EXPONENT > 0.8761);
    }

    #[test]
    fn the_card_is_its_constants_and_its_two_clocks() {
        let link = LinkGothenburg10010::sublink_587();
        assert!(link.is_valid());
        assert_eq!(link.gate_dt_s(), Some(GATE_DT_S));
        assert_eq!(link.floor_dt_s(), Some(60.0 / 1_048_576.0));
        // The ceiling is the account's limit over a thousand, times the
        // wetting time, over the rounding of one part in 2^52.
        let from_arithmetic = ANTENNA_ACCOUNT_LIMIT / 1000.0 * ANTENNA_WETTING_TIME_S
            / f64::EPSILON;
        assert!((GATE_DT_S - from_arithmetic).abs() <= 1.0e-6 * from_arithmetic);
        let mut other = link;
        other.antenna_water_max_db = 2.0;
        assert!(!other.is_valid());
        let mut other = link;
        other.path_length_km = 1.3;
        assert!(!other.is_valid());
    }

    #[test]
    fn free_space_and_the_ideal_aperture_are_the_textbook_numbers() {
        let wavelength_m = 299_792_458.0 / (FREQUENCY_GHZ * 1.0e9);
        let loss = 20.0
            * (4.0 * std::f64::consts::PI * PATH_LENGTH_KM * 1.0e3 / wavelength_m).log10();
        assert!((free_space_loss_db() - loss).abs() < 1.0e-11);
        assert!((free_space_loss_db() - 124.667_182_007).abs() < 1.0e-8);
        let gain = 20.0 * (std::f64::consts::PI * 0.3 / wavelength_m).log10();
        assert!((ideal_aperture_gain_db() - gain).abs() < 1.0e-11);
        assert!((ideal_aperture_gain_db() - 40.172).abs() < 1.0e-3);
    }

    /// The floor of the dry level is the free-space loss less two ideal
    /// apertures, rounded down, so it never refuses a level the two allow.
    #[test]
    fn the_floor_of_the_dry_level_is_free_space_less_two_ideal_apertures() {
        let floor = free_space_loss_db() - 2.0 * ideal_aperture_gain_db();
        assert!((floor - 44.323_253_888).abs() < 1.0e-8, "{floor}");
        assert!(DRY_PATH_LOSS_FLOOR_DB <= floor);
        assert!(floor - DRY_PATH_LOSS_FLOOR_DB < 1.0e-8);
    }

    /// 0 dBm is one milliwatt, 30 dBm one watt, and −46.7 dBm is 21.38 nW.
    #[test]
    fn a_level_in_dbm_is_its_power_in_watts() {
        let link = LinkGothenburg10010::sublink_587();
        assert!((link.power_w_from_level_dbm(0.0) - 1.0e-3).abs() < 1.0e-18);
        assert!((link.power_w_from_level_dbm(30.0) - 1.0).abs() < 1.0e-14);
        assert!((link.power_w_from_level_dbm(-30.0) - 1.0e-6).abs() < 1.0e-20);
        let nanowatts = link.power_w_from_level_dbm(-46.7) * 1.0e9;
        assert!((nanowatts - 10.0_f64.powf(-4.67) * 1.0e6).abs() < 1.0e-12);
        assert!((nanowatts - 21.379_620_895).abs() < 1.0e-8);
    }

    #[test]
    fn a_dry_link_stays_at_its_dry_level() {
        let (link, mut state, mut frame) = dry_start();
        for _ in 0..600 {
            step(&link, &mut state, &mut frame, rain(0.0), MINUTE_S).expect("a dry minute");
        }
        assert_eq!(state.antenna_water_db, 0.0);
        assert_eq!(state.rain_attenuation_db, 0.0);
        assert_eq!(state.path_loss_db, DRY_DB);
        assert_eq!(state.received_level_dbm, SENT_LEVEL_DBM - DRY_DB);
        assert_eq!(frame.flags, 0);
        assert_eq!(frame.t, 36_000.0);
        assert!(state.residual_conservation_error <= 1.0e-15);
        assert_eq!(state.rain_energy_j, 0.0);
        assert_eq!(state.antenna_energy_j, 0.0);
    }

    /// One minute at 12 mm/h from dry, against the equations written out
    /// with the platform's functions.
    #[test]
    fn one_wet_minute_is_the_equations() {
        let (link, mut state, mut frame) = dry_start();
        step(&link, &mut state, &mut frame, rain(12.0), MINUTE_S).expect("a wet minute");
        let rain_db = RAIN_COEFFICIENT_DB_PER_KM * 12.0_f64.powf(RAIN_EXPONENT) * PATH_LENGTH_KM;
        let water_db = ANTENNA_WATER_MAX_DB * (1.0 - (-MINUTE_S / ANTENNA_WETTING_TIME_S).exp());
        assert!((state.rain_attenuation_db - rain_db).abs() < 1.0e-13);
        assert!((state.antenna_water_db - water_db).abs() < 1.0e-14);
        assert!((state.path_loss_db - (DRY_DB + rain_db + water_db)).abs() < 1.0e-12);
        let received_w = 1.0e-3 * 10.0_f64.powf((SENT_LEVEL_DBM - state.path_loss_db) / 10.0);
        assert!((state.received_power_w / received_w - 1.0).abs() < 1.0e-13);
        assert_eq!(frame.flags, FLAG_RAINING);
        assert_eq!(frame.pos[0], state.path_loss_db as f32);
        assert_eq!(frame.pos[1], state.rain_attenuation_db as f32);
        assert_eq!(frame.pos[2], state.antenna_water_db as f32);
        assert_eq!(frame.vel[0], state.received_level_dbm as f32);
        assert_eq!(frame.vel[1], state.received_power_w as f32);
        assert_eq!(frame.vel[2], state.received_energy_j as f32);
        assert_eq!(frame.force_torque, (RAIN_RATE_MAX_MM_H - 12.0) as f32);
        assert_eq!(frame.proof, [0xA5; 16]);
        // 3.172 dB of rain and 1.115 dB of water.
        assert!((state.rain_attenuation_db - 3.172).abs() < 1.0e-3);
        assert!((state.antenna_water_db - 1.1145).abs() < 2.0e-4);
    }

    /// Rain for an hour brings the water to its ceiling and never past it;
    /// two dry hours bring it back under a hundred-millionth of a decibel.
    #[test]
    fn the_antenna_water_builds_to_its_ceiling_and_dries() {
        let (link, mut state, mut frame) = dry_start();
        let mut last = 0.0;
        for _ in 0..60 {
            step(&link, &mut state, &mut frame, rain(6.0), MINUTE_S).expect("a wet minute");
            assert!(state.antenna_water_db >= last);
            assert!(state.antenna_water_db <= ANTENNA_WATER_MAX_DB);
            last = state.antenna_water_db;
        }
        assert!(ANTENNA_WATER_MAX_DB - state.antenna_water_db < 1.0e-12);
        for _ in 0..120 {
            step(&link, &mut state, &mut frame, rain(0.0), MINUTE_S).expect("a dry minute");
            assert!(state.antenna_water_db <= last);
            last = state.antenna_water_db;
        }
        let expected = ANTENNA_WATER_MAX_DB * (-7200.0 / ANTENNA_DRYING_TIME_S).exp();
        assert!((state.antenna_water_db - expected).abs() < 1.0e-15);
        assert!(state.antenna_water_db < 1.0e-7);
        assert!(state.antenna_account_error <= 1.0e-14);
    }

    /// The map is exact: sixty one-second steps land where one minute does.
    #[test]
    fn sixty_seconds_in_sixty_steps_is_one_minute() {
        let (link, mut coarse, mut frame) = dry_start();
        let mut fine = coarse;
        step(&link, &mut coarse, &mut frame, rain(30.0), MINUTE_S).expect("a minute");
        for _ in 0..60 {
            step(&link, &mut fine, &mut frame, rain(30.0), 1.0).expect("a second");
        }
        assert!((coarse.antenna_water_db - fine.antenna_water_db).abs() < 1.0e-14);
        assert_eq!(coarse.rain_attenuation_db, fine.rain_attenuation_db);
        step(&link, &mut coarse, &mut frame, rain(0.0), MINUTE_S).expect("a minute");
        for _ in 0..60 {
            step(&link, &mut fine, &mut frame, rain(0.0), 1.0).expect("a second");
        }
        assert!((coarse.antenna_water_db - fine.antenna_water_db).abs() < 1.0e-14);
    }

    /// A storm that changes every minute for a week of minutes, and for ten
    /// hours of minutes taken a second at a time, and one held rain at the
    /// step floor for 10,080 steps, keeps both ledgers at rounding.
    #[test]
    fn the_ledgers_hold_at_rounding_over_a_long_changing_run() {
        let link = LinkGothenburg10010::sublink_587();
        for (dt_s, steps_a_minute, steps, first_minute) in [
            (MINUTE_S, 1_u32, 10_080_u32, 0_u32),
            (1.0, 60, 36_000, 0),
            (MINUTE_S / 1_048_576.0, 1_048_576, 10_080, 50),
        ] {
            let mut state = LinkState::try_new(&link, DRY_DB, 0.0).expect("a dry start");
            let mut frame = zero_frame();
            let mut worst_energy: f64 = 0.0;
            let mut worst_account: f64 = 0.0;
            for i in 0..steps {
                let minute = first_minute + i / steps_a_minute;
                let phase = (minute % 97) as f64;
                let rate = if minute % 13 < 5 { 0.0 } else { 6.0 + 78.0 * phase / 96.0 };
                step(&link, &mut state, &mut frame, rain(rate), dt_s).expect("on the card");
                worst_energy = worst_energy.max(state.residual_conservation_error);
                worst_account = worst_account.max(state.antenna_account_error);
                assert!(state.antenna_water_db >= 0.0);
                assert!(state.antenna_water_db <= ANTENNA_WATER_MAX_DB);
            }
            assert!(worst_energy < 1.0e-12, "{worst_energy}");
            assert!(worst_account < 1.0e-10, "{worst_account}");
        }
    }

    /// Where the card stood at 84 mm/h, the highest rate of the fitting
    /// data, a rate past it is computed: the rain term is `k R^α L`, the water
    /// builds as at any rate, both ledgers close, and the margin goes below
    /// zero. At 10^300 mm/h the rain takes all the power the dry path
    /// carries, the received power is zero, and the ledger still closes. A
    /// rate below zero is refused by name.
    #[test]
    fn a_rain_rate_past_the_fitted_84_mm_h_is_computed_and_the_ledger_closes() {
        let (link, state, _) = dry_start();
        let mut trial = state;
        let mut frame = zero_frame();
        step(&link, &mut trial, &mut frame, rain(RAIN_RATE_MAX_MM_H), MINUTE_S)
            .expect("84 mm/h lands");
        assert_eq!(frame.force_torque, 0.0);
        for rate in [f64::from_bits(RAIN_RATE_MAX_MM_H.to_bits() + 1), 150.0, 300.0, 1_000.0, 1.0e6] {
            let mut trial = state;
            let mut frame = zero_frame();
            for _ in 0..30 {
                step(&link, &mut trial, &mut frame, rain(rate), MINUTE_S).expect("computed");
                assert!(trial.residual_conservation_error < 1.0e-12, "{rate}");
                assert!(trial.antenna_account_error < 1.0e-12, "{rate}");
            }
            let expected = RAIN_COEFFICIENT_DB_PER_KM * rate.powf(RAIN_EXPONENT) * PATH_LENGTH_KM;
            assert!((trial.rain_attenuation_db - expected).abs() < 1.0e-9 * expected, "{rate}");
            assert!(trial.antenna_water_db <= ANTENNA_WATER_MAX_DB);
            assert!(trial.rain_rate_margin_mm_h < 0.0);
            assert_eq!(frame.flags, FLAG_RAINING);
        }
        // At 300 mm/h the rain alone takes about 50 dB on this path.
        let mut trial = state;
        step(&link, &mut trial, &mut frame, rain(300.0), MINUTE_S).expect("computed");
        assert!(trial.rain_attenuation_db > 40.0 && trial.rain_attenuation_db < 60.0);
        let mut trial = state;
        step(&link, &mut trial, &mut frame, rain(1.0e300), MINUTE_S).expect("all the power");
        assert_eq!(trial.received_power_w, 0.0);
        assert!(trial.residual_conservation_error < 1.0e-12);
        refused(&link, &state, rain(-5.0e-324), MINUTE_S, StepReject::NegativeRainRate);
        refused(&link, &state, rain(-1.0), MINUTE_S, StepReject::NegativeRainRate);
        // Negative zero is zero rain.
        let mut trial = state;
        step(&link, &mut trial, &mut frame, rain(-0.0), MINUTE_S).expect("no rain");
        assert_eq!(frame.flags, 0);
    }

    #[test]
    fn every_input_refusal_is_named_and_leaves_the_state_and_the_proof() {
        let (link, state, _) = dry_start();
        refused(&link, &state, rain(f64::NAN), MINUTE_S, StepReject::NonFiniteInput);
        refused(&link, &state, rain(f64::INFINITY), MINUTE_S, StepReject::NonFiniteInput);
        refused(&link, &state, rain(6.0), f64::NAN, StepReject::NonFiniteInput);
        refused(&link, &state, rain(6.0), f64::NEG_INFINITY, StepReject::NonFiniteInput);
        refused(&link, &state, rain(6.0), 0.0, StepReject::NonPositiveDt);
        refused(&link, &state, rain(6.0), -60.0, StepReject::NonPositiveDt);

        let mut broken = state;
        broken.received_energy_j = f64::NAN;
        refused(&link, &broken, rain(6.0), MINUTE_S, StepReject::NonFiniteState);

        let mut other = link;
        other.antenna_drying_time_s = 405.0;
        refused(&other, &state, rain(6.0), MINUTE_S, StepReject::InvalidLink);

        let mut low = state;
        low.dry_path_loss_db = f64::from_bits(DRY_PATH_LOSS_FLOOR_DB.to_bits() - 1);
        refused(&link, &low, rain(6.0), MINUTE_S, StepReject::DryPathLossBelowFreeSpace);

        let mut soaked = state;
        soaked.antenna_water_db = f64::from_bits(ANTENNA_WATER_MAX_DB.to_bits() + 1);
        refused(&link, &soaked, rain(6.0), MINUTE_S, StepReject::AntennaWaterOutsideCard);
        let mut negative = state;
        negative.antenna_water_db = -1.0e-9;
        refused(&link, &negative, rain(6.0), MINUTE_S, StepReject::AntennaWaterOutsideCard);
    }

    #[test]
    fn a_start_off_the_card_is_not_made() {
        let link = LinkGothenburg10010::sublink_587();
        assert!(LinkState::try_new(&link, DRY_PATH_LOSS_FLOOR_DB, 0.0).is_some());
        assert!(LinkState::try_new(&link, 80.0, ANTENNA_WATER_MAX_DB).is_some());
        assert!(LinkState::try_new(&link, 44.322, 0.0).is_none());
        assert!(LinkState::try_new(&link, DRY_DB, -0.001).is_none());
        assert!(LinkState::try_new(&link, DRY_DB, 2.131).is_none());
        assert!(LinkState::try_new(&link, f64::NAN, 0.0).is_none());
        assert!(LinkState::try_new(&link, DRY_DB, f64::INFINITY).is_none());
        let mut other = link;
        other.sent_level_dbm = 6.0;
        assert!(LinkState::try_new(&other, DRY_DB, 0.0).is_none());
    }

    /// The honest step of this test, to be falsified one field at a time.
    fn honest(
        link: &LinkGothenburg10010,
        state: &LinkState,
        load: LinkLoad,
        dt_s: f64,
    ) -> ExactStep {
        validate(link, state, load, dt_s).expect("valid");
        exact_map(link, state, load, dt_s).expect("finite")
    }

    /// Rule 10. Decibels subtracted as if they were fractions of power, a
    /// loss counted twice, and a loss left out are each named.
    #[test]
    fn a_falsified_power_ledger_is_named() {
        let (link, state, _) = dry_start();
        let load = rain(20.0);
        let dry_energy_j = state.dry_path_power_w * MINUTE_S;

        // The received energy computed as P_dry dt (1 − c (A + W)): a
        // first-order slip that loses energy no term accounts for.
        let mut wrong = honest(&link, &state, load, MINUTE_S);
        wrong.received_energy_j = dry_energy_j
            * (1.0 - NEPER_PER_DB * (wrong.rain_attenuation_db + wrong.antenna_water_db));
        let mut trial = state;
        let mut frame = zero_frame();
        let before = frame.to_bytes();
        assert_eq!(
            commit(&link, &mut trial, &mut frame, load, MINUTE_S, wrong),
            Err(StepReject::FirstLawBreach)
        );
        assert_eq!(trial, state);
        assert_eq!(frame.to_bytes(), before);

        // The water's share taken from the dry-path energy and not from what
        // the rain left: the rain's loss counted twice over.
        let mut wrong = honest(&link, &state, load, MINUTE_S);
        let past_rain = (-NEPER_PER_DB * wrong.rain_attenuation_db).exp();
        wrong.antenna_energy_j /= past_rain;
        let mut trial = state;
        assert_eq!(
            commit(&link, &mut trial, &mut frame, load, MINUTE_S, wrong),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(trial, state);
        assert_eq!(frame.to_bytes(), before);

        // A received energy one part in a hundred million high.
        let mut wrong = honest(&link, &state, load, MINUTE_S);
        wrong.received_energy_j *= 1.0 + 1.0e-8;
        let mut trial = state;
        assert_eq!(
            commit(&link, &mut trial, &mut frame, load, MINUTE_S, wrong),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(trial, state);

        // The rain's share left out.
        let mut wrong = honest(&link, &state, load, MINUTE_S);
        wrong.rain_energy_j = 0.0;
        let mut trial = state;
        assert_eq!(
            commit(&link, &mut trial, &mut frame, load, MINUTE_S, wrong),
            Err(StepReject::FirstLawBreach)
        );
        assert_eq!(trial, state);
        assert_eq!(frame.to_bytes(), before);

        // The honest step lands.
        let mut trial = state;
        let right = honest(&link, &state, load, MINUTE_S);
        assert_eq!(commit(&link, &mut trial, &mut frame, load, MINUTE_S, right), Ok(()));
        assert!(trial.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    /// The energies an end-of-step power held through the step gives, the
    /// error of engine 0.6.0, still sum to the dry-path energy: the ledger
    /// lands them. What pins the division is the integral, which the next
    /// test checks against quadrature.
    #[test]
    fn the_ledger_does_not_divide_the_energy_between_receiver_and_antennas() {
        let (link, state, _) = dry_start();
        let load = rain(12.0);
        let mut held = honest(&link, &state, load, MINUTE_S);
        let past_rain = (-NEPER_PER_DB * held.rain_attenuation_db).exp();
        held.received_energy_j = held.received_power_w * MINUTE_S;
        held.antenna_energy_j = state.dry_path_power_w
            * past_rain
            * (1.0 - (-NEPER_PER_DB * held.antenna_water_db).exp())
            * MINUTE_S;
        let mut trial = state;
        let mut frame = zero_frame();
        assert_eq!(commit(&link, &mut trial, &mut frame, load, MINUTE_S, held), Ok(()));
    }

    /// Each energy of a step is its power integrated over the step: against
    /// a Simpson quadrature of the card's own powers with 20,000 panels, at
    /// a wet minute from dry, a wet minute from near the ceiling, a dry
    /// minute from the ceiling, and a wet second.
    #[test]
    fn each_energy_is_its_power_integrated_over_the_step() {
        let link = LinkGothenburg10010::sublink_587();
        for (start_db, rate, dt_s) in [
            (0.0, 12.0, MINUTE_S),
            (2.0, 84.0, MINUTE_S),
            (ANTENNA_WATER_MAX_DB, 0.0, MINUTE_S),
            (0.7, 36.0, 1.0),
        ] {
            let state = LinkState::try_new(&link, DRY_DB, start_db).expect("a start");
            let map = honest(&link, &state, rain(rate), dt_s);
            let raining = rate > 0.0;
            let water = |t: f64| {
                if raining {
                    ANTENNA_WATER_MAX_DB
                        - (ANTENNA_WATER_MAX_DB - start_db) * (-t / ANTENNA_WETTING_TIME_S).exp()
                } else {
                    start_db * (-t / ANTENNA_DRYING_TIME_S).exp()
                }
            };
            let rain_db = map.rain_attenuation_db;
            let p_dry = state.dry_path_power_w;
            let received = |t: f64| p_dry * (-NEPER_PER_DB * (rain_db + water(t))).exp();
            let antennas = |t: f64| {
                p_dry * (-NEPER_PER_DB * rain_db).exp() * (1.0 - (-NEPER_PER_DB * water(t)).exp())
            };
            let simpson = |f: &dyn Fn(f64) -> f64| {
                let panels = 20_000;
                let h = dt_s / panels as f64;
                let mut sum = f(0.0) + f(dt_s);
                for k in 1..panels {
                    let weight = if k % 2 == 1 { 4.0 } else { 2.0 };
                    sum += weight * f(k as f64 * h);
                }
                sum * h / 3.0
            };
            let received_j = simpson(&received);
            let antennas_j = simpson(&antennas);
            assert!((map.received_energy_j / received_j - 1.0).abs() < 1.0e-13, "{start_db} {rate}");
            if antennas_j > 0.0 {
                assert!((map.antenna_energy_j / antennas_j - 1.0).abs() < 1.0e-12, "{start_db} {rate}");
            }
            let rain_j = p_dry * (1.0 - (-NEPER_PER_DB * rain_db).exp()) * dt_s;
            assert!((map.rain_energy_j - rain_j).abs() <= 1.0e-15 * p_dry * dt_s);
        }
    }

    /// One step and many steps over the same span give the same energies,
    /// where engine 0.6.0 gave different ones: one minute at 12 mm/h from
    /// dry, and sixty seconds; then a dry minute after it.
    #[test]
    fn one_step_and_sixty_steps_give_the_same_energies() {
        let (link, mut coarse, mut frame) = dry_start();
        let mut fine = coarse;
        for rate in [12.0, 0.0] {
            step(&link, &mut coarse, &mut frame, rain(rate), MINUTE_S).expect("a minute");
            for _ in 0..60 {
                step(&link, &mut fine, &mut frame, rain(rate), 1.0).expect("a second");
            }
            for (one, many) in [
                (coarse.received_energy_j, fine.received_energy_j),
                (coarse.rain_energy_j, fine.rain_energy_j),
                (coarse.antenna_energy_j, fine.antenna_energy_j),
                (coarse.dry_path_energy_j, fine.dry_path_energy_j),
            ] {
                assert!((one / many - 1.0).abs() < 1.0e-13, "{one} {many}");
            }
        }
        // The integral of the card's own powers over the first minute, worked
        // outside this crate: 4.67295e-7 J received and 7.0908e-8 J on the
        // antennas, at a dry level of 52.3 dB.
        let (link, mut state, mut frame) = dry_start();
        step(&link, &mut state, &mut frame, rain(12.0), MINUTE_S).expect("a minute");
        assert!((state.received_energy_j / 4.67295e-7 - 1.0).abs() < 1.0e-5, "{}", state.received_energy_j);
        assert!((state.antenna_energy_j / 7.0908e-8 - 1.0).abs() < 1.0e-4, "{}", state.antenna_energy_j);
        assert_eq!(frame.vel[2], state.received_energy_j as f32);
    }

    /// Rule 10. The antenna water stepped by forward Euler, wetting and
    /// drying, with the power received and the energies computed from that
    /// wrong water and held through the step, so that the ledger still
    /// closes, is named by the account and by nothing else.
    #[test]
    fn an_euler_step_of_the_antenna_water_is_named() {
        let link = LinkGothenburg10010::sublink_587();
        let falsify = |wrong: &mut ExactStep, state: &LinkState, water_db: f64| {
            let rain_db = wrong.rain_attenuation_db;
            let past_rain = (-NEPER_PER_DB * rain_db).exp();
            wrong.antenna_water_db = water_db;
            wrong.received_power_w =
                state.dry_path_power_w * (-NEPER_PER_DB * (rain_db + water_db)).exp();
            wrong.received_energy_j = wrong.received_power_w * MINUTE_S;
            wrong.antenna_energy_j = state.dry_path_power_w
                * past_rain
                * (1.0 - (-NEPER_PER_DB * water_db).exp())
                * MINUTE_S;
        };
        for (start_db, load, time_s) in [
            (0.0, rain(20.0), ANTENNA_WETTING_TIME_S),
            (2.0, rain(0.0), ANTENNA_DRYING_TIME_S),
        ] {
            let state = LinkState::try_new(&link, DRY_DB, start_db).expect("a start");
            let mut wrong = honest(&link, &state, load, MINUTE_S);
            let target_db = if load.rain_rate_mm_h > 0.0 { ANTENNA_WATER_MAX_DB } else { 0.0 };
            let euler_db = start_db + (target_db - start_db) * MINUTE_S / time_s;
            falsify(&mut wrong, &state, euler_db);
            let mut trial = state;
            let mut frame = zero_frame();
            let before = frame.to_bytes();
            assert_eq!(
                commit(&link, &mut trial, &mut frame, load, MINUTE_S, wrong),
                Err(StepReject::AntennaAccountBreach)
            );
            assert_eq!(trial, state);
            assert_eq!(frame.to_bytes(), before);
        }
        // An antenna water one part in a hundred thousand off the map is named too.
        let state = LinkState::try_new(&link, DRY_DB, 1.0).expect("a start");
        let mut wrong = honest(&link, &state, rain(6.0), MINUTE_S);
        let off_db = wrong.antenna_water_db + 1.0e-5 * ANTENNA_WATER_MAX_DB;
        falsify(&mut wrong, &state, off_db);
        let mut trial = state;
        let mut frame = zero_frame();
        assert_eq!(
            commit(&link, &mut trial, &mut frame, rain(6.0), MINUTE_S, wrong),
            Err(StepReject::AntennaAccountBreach)
        );
        assert_eq!(trial, state);
        assert!(ANTENNA_ACCOUNT_LIMIT < 1.0e-5);
    }

    /// Where the card stood below one gauge count a minute, 6 mm/h, a light
    /// rain is computed: the rain term is `k R^α L` at that rate, the water
    /// builds toward its ceiling as at any rate above zero, and both ledgers
    /// close. Zero is no rain and the water dries.
    #[test]
    fn a_rain_rate_under_one_gauge_count_is_computed_and_the_ledger_closes() {
        let (link, state, _) = dry_start();
        for rate in [5.0e-324, 1.0e-30, 0.1, 1.0, 5.999_999_999_999_999] {
            let mut trial = state;
            let mut frame = zero_frame();
            for _ in 0..20 {
                step(&link, &mut trial, &mut frame, rain(rate), MINUTE_S).expect("computed");
                assert!(trial.residual_conservation_error < 1.0e-12, "{rate}");
                assert!(trial.antenna_account_error < 1.0e-12, "{rate}");
            }
            assert_eq!(frame.flags, FLAG_RAINING);
            assert!(trial.rain_attenuation_db < 2.0, "{rate}");
            // The antenna term does not depend on the rate.
            assert!((trial.antenna_water_db - ANTENNA_WATER_MAX_DB).abs() < 1.0e-5);
            assert!(LinkState::try_continue(&link, DRY_DB, 0.0, 0.0, rate).is_ok());
        }
        let mut trial = state;
        let mut frame = zero_frame();
        step(&link, &mut trial, &mut frame, rain(0.0), MINUTE_S).expect("no rain lands");
        assert_eq!(trial.antenna_water_db, 0.0);
    }

    /// Where the card stood when the rain changed inside a gauge minute, the
    /// step is computed: the map is exact for the rate each step holds, so
    /// ten seconds at one rate and fifty at another land where the same two
    /// pieces land taken in any number of steps, and both ledgers close.
    #[test]
    fn rain_changed_inside_a_gauge_minute_is_computed_and_the_ledger_closes() {
        let (link, start, mut frame) = dry_start();
        let mut state = start;
        step(&link, &mut state, &mut frame, rain(36.0), 10.0).expect("ten seconds");
        assert_eq!(state.gauge_minute_elapsed_s, 10.0);
        assert_eq!(state.gauge_minute_rain_mm_h, 36.0);
        step(&link, &mut state, &mut frame, rain(0.0), 10.0).expect("a change inside the minute");
        step(&link, &mut state, &mut frame, rain(42.0), 40.0).expect("another");
        assert_eq!(state.gauge_minute_elapsed_s, 0.0);
        assert!(state.residual_conservation_error < 1.0e-12);
        assert!(state.antenna_account_error < 1.0e-12);
        let mut fine = start;
        for _ in 0..10 {
            step(&link, &mut fine, &mut frame, rain(36.0), 1.0).expect("a second");
        }
        for _ in 0..100 {
            step(&link, &mut fine, &mut frame, rain(0.0), 0.1).expect("a tenth");
        }
        for _ in 0..8 {
            step(&link, &mut fine, &mut frame, rain(42.0), 5.0).expect("five seconds");
        }
        assert!((fine.antenna_water_db - state.antenna_water_db).abs() < 1.0e-13);
        assert!((fine.received_energy_j - state.received_energy_j).abs()
            < 1.0e-12 * state.received_energy_j);
        // The rain changing every second for an hour keeps both ledgers.
        let mut state = start;
        for i in 0..3600_u32 {
            let rate = [0.0, 3.0, 18.0, 120.0, 0.5][(i % 5) as usize];
            step(&link, &mut state, &mut frame, rain(rate), 1.0).expect("each second");
        }
        assert!(state.residual_conservation_error < 1.0e-12);
        assert!(state.antenna_account_error < 1.0e-10);
    }

    /// Where the card stood at the gauge's 60 s step, one step of a day, a
    /// year, or the ceiling lands where many steps land, and both ledgers
    /// close; past the ceiling the antenna account still closes for one step,
    /// and its rounding grows as the step does.
    #[test]
    fn a_step_far_past_the_gauge_minute_lands_where_many_steps_do() {
        let (link, start, mut frame) = dry_start();
        for (dt_s, steps) in [(3_600.0, 24_u32), (86_400.0, 1), (MINUTE_S * 7.0, 13)] {
            let mut one = start;
            step(&link, &mut one, &mut frame, rain(24.0), dt_s * f64::from(steps)).expect("one");
            let mut many = start;
            for _ in 0..steps * 60 {
                step(&link, &mut many, &mut frame, rain(24.0), dt_s / 60.0).expect("many");
            }
            assert!((one.antenna_water_db - many.antenna_water_db).abs() < 1.0e-13);
            assert!((one.received_energy_j - many.received_energy_j).abs()
                < 1.0e-12 * one.received_energy_j);
            assert!(one.residual_conservation_error < 1.0e-12);
            assert!(many.antenna_account_error < 1.0e-10);
        }
        for dt_s in [86_400.0 * 365.25, GATE_DT_S] {
            for rate in [0.0, 24.0] {
                let mut one = LinkState::try_new(&link, DRY_DB, 1.0).expect("a start");
                step(&link, &mut one, &mut frame, rain(rate), dt_s).expect("at the ceiling");
                assert!(one.residual_conservation_error < 1.0e-12, "{dt_s}");
                assert!(one.antenna_account_error <= ANTENNA_ACCOUNT_LIMIT / 1000.0, "{dt_s} {}", one.antenna_account_error);
            }
        }
    }

    /// Restart family. A state started part way through a minute takes any
    /// rate at its first step; a start off the minute's clock, or with a rate
    /// below zero, is refused by name.
    #[test]
    fn a_start_part_way_through_a_minute_takes_any_rain() {
        let link = LinkGothenburg10010::sublink_587();
        let state = LinkState::try_continue(&link, DRY_DB, 0.3, 30.0, 36.0).expect("a start");
        assert_eq!(state.t_s, 0.0);
        let mut trial = state;
        let mut frame = zero_frame();
        step(&link, &mut trial, &mut frame, rain(0.0), 10.0).expect("another rate");
        assert_eq!(trial.gauge_minute_elapsed_s, 40.0);
        assert_eq!(trial.gauge_minute_rain_mm_h, 0.0);
        for elapsed in [-1.0e-300, MINUTE_S, 61.0] {
            assert_eq!(
                LinkState::try_continue(&link, DRY_DB, 0.0, elapsed, 0.0),
                Err(StepReject::GaugeMinuteOutsideCard)
            );
        }
        assert_eq!(
            LinkState::try_continue(&link, DRY_DB, 0.0, f64::NAN, 0.0),
            Err(StepReject::NonFiniteState)
        );
        assert_eq!(
            LinkState::try_continue(&link, DRY_DB, 0.0, 0.0, -0.5),
            Err(StepReject::NegativeRainRate)
        );
        assert!(LinkState::try_continue(&link, DRY_DB, 0.0, 0.0, 84.5).is_ok());
        let last = f64::from_bits(MINUTE_S.to_bits() - 1);
        assert!(LinkState::try_continue(&link, DRY_DB, 0.0, last, 0.0).is_ok());
        // A state written by hand with its minute off the card is refused at the step.
        let (_, mut bad, _) = dry_start();
        bad.gauge_minute_elapsed_s = MINUTE_S;
        refused(&link, &bad, rain(0.0), 1.0, StepReject::GaugeMinuteOutsideCard);
        let (_, mut bad, _) = dry_start();
        bad.gauge_minute_rain_mm_h = -3.0;
        refused(&link, &bad, rain(0.0), 1.0, StepReject::NegativeRainRate);
    }

    /// Where the card stood at 50 and 54 dB, any dry level from the free-space
    /// floor up is computed with both ledgers closed; under the floor it is
    /// refused by name.
    #[test]
    fn a_dry_level_from_the_free_space_floor_up_is_computed() {
        let link = LinkGothenburg10010::sublink_587();
        for dry_db in [DRY_PATH_LOSS_FLOOR_DB, 45.0, 49.0, 55.0, 70.0, 140.0] {
            let mut state = LinkState::try_new(&link, dry_db, 0.0).expect("a start");
            let mut frame = zero_frame();
            for rate in [0.0, 12.0, 84.0, 200.0, 0.0] {
                step(&link, &mut state, &mut frame, rain(rate), MINUTE_S).expect("computed");
                assert!(state.residual_conservation_error < 1.0e-12, "{dry_db}");
                assert!(state.antenna_account_error < 1.0e-12, "{dry_db}");
            }
            assert!((state.path_loss_db - dry_db - state.antenna_water_db).abs() < 1.0e-12 * dry_db);
        }
        let below = f64::from_bits(DRY_PATH_LOSS_FLOOR_DB.to_bits() - 1);
        assert_eq!(
            LinkState::try_continue(&link, below, 0.0, 0.0, 0.0),
            Err(StepReject::DryPathLossBelowFreeSpace)
        );
        assert_eq!(
            LinkState::try_continue(&link, -10.0, 0.0, 0.0, 0.0),
            Err(StepReject::DryPathLossBelowFreeSpace)
        );
        // A dry level so large that the dry path carries no power in 64 bits
        // is refused as not finite, not computed.
        let mut state = LinkState::try_new(&link, DRY_DB, 0.0).expect("a start");
        state.dry_path_loss_db = 1.0e4;
        state.dry_path_power_w = 0.0;
        refused(&link, &state, rain(6.0), MINUTE_S, StepReject::NonFiniteState);
    }

    /// A trial step that is not a number is refused before it is written.
    #[test]
    fn a_non_finite_trial_step_is_named() {
        let (link, state, _) = dry_start();
        let mut wrong = honest(&link, &state, rain(6.0), MINUTE_S);
        wrong.antenna_water_integral_db_s = f64::NAN;
        let mut trial = state;
        let mut frame = zero_frame();
        assert_eq!(
            commit(&link, &mut trial, &mut frame, rain(6.0), MINUTE_S, wrong),
            Err(StepReject::NonFiniteState)
        );
        assert_eq!(trial, state);
    }

    /// The first twelve minutes of the fitted event that starts 2015-07-28T14:26:00Z, against `analysis/out/card_reference.json`
    /// (plain Python, the forecast's own model): excess attenuation in dB.
    #[test]
    fn a_fitted_event_follows_the_forecast_model() {
        let (link, _, mut frame) = dry_start();
        let mut state = LinkState::try_new(&link, REFERENCE_DRY_DB, 0.0).expect("a dry start");
        for (rate, excess_db) in REFERENCE_MINUTES {
            step(&link, &mut state, &mut frame, rain(rate), MINUTE_S).expect("on the card");
            let got = state.path_loss_db - REFERENCE_DRY_DB;
            assert!((got - excess_db).abs() < 1.0e-12, "{got} {excess_db}");
        }
    }

    const REFERENCE_DRY_DB: f64 = 52.325;
    const REFERENCE_MINUTES: [(f64, f64); 12] = [
        (6.0, 2.8205208827748818),
        (18.0, 6.205202544543754),
        (18.0, 6.458529139795127),
        (18.0, 6.579305286605741),
        (6.0, 3.7835545908694073),
        (6.0, 3.8110070932263325),
        (6.0, 3.8240953655085557),
        (6.0, 3.83033533843002),
        (18.0, 6.686642319260911),
        (6.0, 3.8347286620375485),
        (6.0, 3.8354048755680052),
        (12.0, 5.301757370842308),
    ];
}
