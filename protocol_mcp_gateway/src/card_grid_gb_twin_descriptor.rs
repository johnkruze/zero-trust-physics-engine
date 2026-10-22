//! What an agent reads of the grid_gb_twin card: its fields and bounds, its
//! step floor and ceiling, its refusals word for word from the card's source,
//! its frame, its worked examples and its record. A child module of
//! `descriptor.rs`, so it uses that file's own builders.

use genesis_kernel::grid_gb_twin;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_recorded, scalar, scalar_value, slots, token};

const DOMAIN: &str = "energy";
const CARD: &str = "grid_gb";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "The Great Britain synchronous area, the 50 Hz transmission system of England, Scotland and Wales as one rotating mass, after an importing interconnector trips, as the system of the day: the inertia the operator published for the half hour and the Dynamic Containment, Moderation and Regulation it held in that four-hour block, on NESO's own delivery curves. Compared on all 86 trips of 500 to 1402 MW, May 2017 to June 2026.";

/// The result on the trips, in numbers. Written from
/// `evidence/research/grid_gb_interconnector_loss/twin/out/engine_scores.json`.
pub(super) const RESULT: &str = "Through the engine, with the five constants fitted on all 86: depth of the fall 0.217 RMS relative error, mean +0.012; steepest second 0.236; frequency near 60 s 0.075 Hz; whole trace 0.035 Hz median. All 86 runs complete, each tape verifies, the largest residual 3.4e-11. With each trip left out of the fit in turn and the five refitted on the other 85: depth 0.224, mean +0.014, from 64 percent too shallow to 106 percent too deep; steepest second 0.237; near 60 s 0.076 Hz; trace 0.035 Hz. By period, each trip left out in turn: 2017 to 2020, 22 trips, 0.220, mean -0.012; 2021 to 2023, 30 trips, 0.193; 2024 to October 2025, 23 trips, 0.175, mean -0.004; since November 2025, 11 trips, 0.359, mean +0.233, and without 21 March 2026, 0.174. By loss: 500 to 692 MW, 9 trips, 0.275; 693 to 999 MW, 52 trips, 0.202; 1000 to 1402 MW, 25 trips, 0.245. On the 40 trips first compared, 0.189. The worked minute, 17 November 2025, 1007 MW from 49.948 Hz: lowest 49.752862 Hz against 49.753 measured.";

/// The limits of the comparison.
pub(super) const LIMITS: &str = "The inertia is NESO's outturn estimate, which leaves out the Stability Pathfinder units, 11 to 15 synchronous compensators available from September 2023, and grid-forming batteries; with an inertia offset fitted as well, the depth error on all 86 does not move, 0.221. The volumes are the cleared auction volumes of the four-hour block, not the delivery metered on the day. Dynamic Containment is held at its contracted volume: with its share free the fit sets it to zero, because below 49.8 Hz its curve and the plant's knee are the same shape on these trips. The fit gives Dynamic Moderation 0.913 of its volume and Dynamic Regulation none: within a minute its ten-second delivery is not told apart from the plant's own response. Before October 2020 no dynamic service is in the data, and the response held then, firm and mandatory frequency response, is the plant term, one for every year. Fifteen structures were fitted on all 86, depth 0.210 to 0.312. With the delivery times free the fit drives them to 0.05 s, faster than NESO's 0.5 s initiation (0.210), so they are held at NESO's times; with every share free (0.215; left out in turn 0.220, and 0.287 since November 2025) it sets Containment's to zero and the calendar comes back, a mean of -0.135 in 2017 to 2020 and +0.061 in 2024 to October 2025, so Containment is held as contracted (0.217); every service as contracted, three constants, gives 0.221, and left out in turn 0.226, 0.271 since November 2025 but 0.381 on 500 to 692 MW and a trace of 0.042 Hz; the earlier structure, one response channel steeper below a knee and no service curves, refitted on the 86 gives 0.312, and 0.319 left out in turn. The steepest second reads the first samples of a fall at one value a second; since November 2025 it is 0.45 RMS. Four trips are past 0.6 either way, each left out in turn: 21 March 2026, 1002 MW from 50.110 Hz with a measured fall of 0.142 Hz, 106 percent too deep; 4 March 2024, 960 MW with a fall of 0.141 Hz, 69 percent; 26 March 2018, 634 MW, 68 percent; and 22 December 2023, 64 percent too shallow, where a second fall begins eight seconds after the first, a loss the open data does not name. One frequency for the whole area; no network, no generator that trips as a consequence and no reserve slower than these services, so a step past 60 s is flagged. One party wrote the card, fitted it and ran it, on one machine.";

pub(super) fn card() -> Card {
    let specimen = grid_gb_twin::GridGbTwin::great_britain();
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let volume = |name: &'static str, service: &str, largest: f64| {
        scalar(
            name,
            Role::Phase,
            "MW",
            Some(0.0),
            false,
            None,
            false,
            format!(
                "at least 0 MW (ServiceVolumeNegative): {service} cleared for the block. Compared 0 to {} MW; above sets FLAG_OUTSIDE_COMPARED_TRIPS",
                token(largest)
            ),
        )
    };
    let delivery = |name: &'static str, services: &str, compared: f64| {
        scalar(
            name,
            Role::Phase,
            "s",
            Some(0.0),
            true,
            None,
            false,
            format!(
                "above 0 s (DeliveryTimeNotPositive): time to full delivery of {services}, a lag of a third of it. Compared at {} s; another sets FLAG_OUTSIDE_COMPARED_TRIPS",
                token(compared)
            ),
        )
    };
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "frequency_hz",
                Role::Phase,
                "Hz",
                Some(grid_gb_twin::FREQUENCY_MIN_HZ),
                true,
                Some(grid_gb_twin::FREQUENCY_MAX_HZ),
                false,
                format!(
                    "above {} Hz and at most {} Hz, the system frequency at the start, with the area in balance: at or below 48.8 Hz low-frequency demand disconnection has begun (LowFrequencyDemandDisconnection), and above 50.5 Hz is the statutory limit (FrequencyAboveCard). Compared on starts of 49.763 to 50.126 Hz, rounded outward to {} to {} Hz; a start outside sets FLAG_OUTSIDE_COMPARED_TRIPS",
                    token(grid_gb_twin::FREQUENCY_MIN_HZ),
                    token(grid_gb_twin::FREQUENCY_MAX_HZ),
                    token(grid_gb_twin::START_FREQUENCY_MIN_HZ),
                    token(grid_gb_twin::START_FREQUENCY_MAX_HZ)
                ),
            ),
            scalar(
                "inertia_mw_s",
                Role::Phase,
                "MW*s",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "above 0 MW s (InertiaNotPositive), the kinetic energy the area stores at 50 Hz: NESO's published outturn inertia for the half hour, in GVA s, times 1000. Compared on {} to {} MW s; outside it sets FLAG_OUTSIDE_COMPARED_TRIPS, and the run's step ceiling is recomputed from its own inertia",
                    token(grid_gb_twin::INERTIA_MIN_MW_S),
                    token(grid_gb_twin::INERTIA_MAX_MW_S)
                ),
            ),
            scalar(
                "demand_mw",
                Role::Phase,
                "MW",
                Some(0.0),
                false,
                None,
                false,
                format!(
                    "at least 0 MW (DemandNegative), generation and net imports on the transmission system just before the loss, the sum of Elexon's FUELINST categories. Compared on {} to {} MW; outside it sets FLAG_OUTSIDE_COMPARED_TRIPS",
                    token(grid_gb_twin::DEMAND_MIN_MW),
                    token(grid_gb_twin::DEMAND_MAX_MW)
                ),
            ),
            volume("dc_low_mw", "DCL", grid_gb_twin::DC_LOW_MAX_MW),
            volume("dm_low_mw", "DML", grid_gb_twin::DM_LOW_MAX_MW),
            volume("dr_low_mw", "DRL", grid_gb_twin::DR_LOW_MAX_MW),
            volume("dc_high_mw", "DCH", grid_gb_twin::DC_HIGH_MAX_MW),
            volume("dm_high_mw", "DMH", grid_gb_twin::DM_HIGH_MAX_MW),
            volume("dr_high_mw", "DRH", grid_gb_twin::DR_HIGH_MAX_MW),
            delivery("fast_delivery_s", "DC and DM", grid_gb_twin::FAST_DELIVERY_COMPARED_S),
            delivery("regulation_delivery_s", "DR", grid_gb_twin::REGULATION_DELIVERY_COMPARED_S),
            scalar(
                "loss_mw",
                Role::Load,
                "MW",
                None,
                false,
                None,
                false,
                format!(
                    "the power lost, held through each step: a loss of infeed when positive, a loss of demand when negative, never larger in size than demand_mw (LossLargerThanDemand). Compared on {} to {} MW, each one loss held from time zero; a loss outside that span, or one that is not the run's first, sets FLAG_OUTSIDE_COMPARED_TRIPS",
                    token(grid_gb_twin::LOSS_MIN_MW),
                    token(grid_gb_twin::LOSS_MAX_MW)
                ),
            ),
        ],
        floor,
        "a locked 1e-6 s: a step of that size still moves the clock at 60 s",
        gate,
        "0.02 of the fastest time scale the card can reach at 48.8 Hz: a service's lag, the plant's lag, the relief pole of the demand, or the swing of the rotating mass against a response. The value here is at the corner of the compared trips, 111,000 MW s, 43,106 MW and every volume at its largest, at NESO's delivery times. A run takes the same rule at its own inertia, demand, volumes and delivery times where that gives a smaller ceiling, and a step above it is refused as DtAboveCeiling with the ceiling stated",
        None,
        format!(
            "frequency_hz, inertia_mw_s, demand_mw, the six volumes and the two delivery times are the area's condition at the start and are held through the run; loss_mw is the load of each segment. A run starts at time zero with the area in balance at frequency_hz and no response delivered beyond what the start frequency already calls for. Below 50 Hz the low volumes deliver and above it the high volumes absorb, each on NESO's curve: a deadband of {} Hz; Dynamic Containment 5 percent at 0.2 Hz from 50 Hz and all of it at 0.5 Hz; Dynamic Moderation 5 percent at 0.1 Hz and all of it at 0.2 Hz; Dynamic Regulation in a straight line to all of it at 0.2 Hz. The plant's own response is {} MW/Hz below 50 Hz and a further {} MW/Hz below {} Hz, through a lag of {} s; demand's relief is {} of demand per hertz. The share of each contracted volume the area receives: Dynamic Containment {}, as contracted; Dynamic Moderation {}, fitted; Dynamic Regulation {}, fitted. A single step starts in the same way at each call, so single steps cannot be chained: a run is the way to follow the fall. A step that lands later than 60 s sets FLAG_PAST_COMPARED_MINUTE, and one under 49.55 Hz, the lowest single-loss frequency compared, FLAG_BELOW_COMPARED_FREQUENCY. The clock is a running sum, t_s += dt_s in 64-bit arithmetic, and the comparison with 60 s carries an allowance of 3.55e-15 s for each step a run takes to reach it, never more than half the step. A step of zero or less is refused as NonPositiveDt",
            token(grid_gb_twin::SERVICE_DEADBAND_HZ),
            token(grid_gb_twin::PLANT_GAIN_MW_PER_HZ),
            token(grid_gb_twin::PLANT_GAIN_BELOW_KNEE_MW_PER_HZ),
            token(grid_gb_twin::PLANT_KNEE_HZ),
            token(grid_gb_twin::PLANT_RESPONSE_TIME_S),
            token(grid_gb_twin::LOAD_RELIEF_PER_HZ),
            token(grid_gb_twin::DELIVERY_SHARE_DC),
            token(grid_gb_twin::DELIVERY_SHARE_DM),
            token(grid_gb_twin::DELIVERY_SHARE_DR),
        ),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCard",
            "InertiaNotPositive",
            "DemandNegative",
            "ServiceVolumeNegative",
            "DeliveryTimeNotPositive",
            "LossLargerThanDemand",
            "NegativeTime",
            "LowFrequencyDemandDisconnection",
            "FrequencyAboveCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory { name: "FLAG_OUTSIDE_COMPARED_TRIPS", mask: grid_gb_twin::FLAG_OUTSIDE_COMPARED_TRIPS, seals: true },
            Advisory { name: "FLAG_PAST_COMPARED_MINUTE", mask: grid_gb_twin::FLAG_PAST_COMPARED_MINUTE, seals: true },
            Advisory { name: "FLAG_BELOW_COMPARED_FREQUENCY", mask: grid_gb_twin::FLAG_BELOW_COMPARED_FREQUENCY, seals: true },
        ],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "frequency_hz"),
            ("pos[1]", "plant_response_mw"),
            ("pos[2]", "service_response_mw"),
            ("vel[0]", "frequency_rate_hz_per_s"),
            ("vel[1]", "lowest_frequency_hz"),
            ("vel[2]", "lowest_time_s"),
            ("force_torque", "Net power on the rotating mass, in MW: the responses plus relief less the loss."),
            ("residual", "Conservation residual."),
            (
                "flags",
                "FLAG_OUTSIDE_COMPARED_TRIPS when the start, the response held or the step's loss is outside the compared trips or the loss is not the run's first; FLAG_PAST_COMPARED_MINUTE when the step lands later than 60 s; FLAG_BELOW_COMPARED_FREQUENCY when it lands under 49.55 Hz. Each seals.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One step at the ceiling, and a minute of the trip of 17 November 2025,
/// 06:19:28 UTC: IFA, 1007 MW, 177 GVA s, 26,774 MW, from 49.948 Hz, with
/// the volumes NESO held in that block. Measured lowest 49.753 Hz.
pub(super) fn examples() -> (Value, Value) {
    let specimen = grid_gb_twin::GridGbTwin::great_britain();
    let dt = specimen.gate_dt_s().expect("specimen gate");
    let steps = (60.0 / dt) as u64 + 1;
    pair_examples_recorded(
        DOMAIN,
        CARD,
        obj(&[
            ("frequency_hz", scalar_value(49.948)),
            ("inertia_mw_s", scalar_value(177_000.0)),
            ("demand_mw", scalar_value(26_774.0)),
            ("dc_low_mw", scalar_value(1145.0)),
            ("dm_low_mw", scalar_value(500.0)),
            ("dr_low_mw", scalar_value(480.0)),
            ("dc_high_mw", scalar_value(1080.0)),
            ("dm_high_mw", scalar_value(510.0)),
            ("dr_high_mw", scalar_value(480.0)),
            ("fast_delivery_s", scalar_value(1.0)),
            ("regulation_delivery_s", scalar_value(10.0)),
        ]),
        obj(&[("loss_mw", scalar_value(1007.0))]),
        super::card_dt(Some(60.0 / steps as f64)),
        steps,
        steps / 60,
    )
}

/// The record, in the five parts every card gives: the object, the data,
/// the measure, the result and the limits.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        "The Great Britain synchronous area, the 50 Hz transmission system of England, Scotland and Wales, in the 60 seconds after an importing interconnector, or one pole of it, trips: 86 trips of 500 to 1402 MW between May 2017 and June 2026, every single-interconnector loss of 500 MW or more that the scan of the grid_gb record found, with its rule applied to all 1,160 falls of 0.12 Hz or more in eight seconds; one trip, 2 September 2021, is dropped by its quality rule.".into(),
        format!(
            "NESO Open Data: the system frequency once a second, the half-hourly outturn inertia, and the cleared volumes of Dynamic Containment (from October 2020), Dynamic Moderation and Dynamic Regulation (from 2022), low and high, by four-hour block (Dynamic Containment Masterdata; DC, DR & DM Results Summary 2021-2023; EAC Response-Reserve Results Summary from November 2023); Elexon BMRS FUELINST for each loss and the demand. Fitted on all 86 trips at once, least squares on the frequency sample by sample over 60 s: the plant's response, {} MW/Hz below 50 Hz and a further {} MW/Hz below 49.8 Hz through a lag of {} s, and the shares delivered of Dynamic Moderation, {}, and Dynamic Regulation, {}. Not fitted: Dynamic Containment delivers as contracted; the service curves and delivery times are NESO's; demand's relief is 2.5 percent per hertz.",
            token(grid_gb_twin::PLANT_GAIN_MW_PER_HZ),
            token(grid_gb_twin::PLANT_GAIN_BELOW_KNEE_MW_PER_HZ),
            token(grid_gb_twin::PLANT_RESPONSE_TIME_S),
            token(grid_gb_twin::DELIVERY_SHARE_DM),
            token(grid_gb_twin::DELIVERY_SHARE_DR),
        ),
        "Depth of the fall (the start sample less the lowest of the next 60) and the steepest second (the largest fall between consecutive samples in the first five): root mean square of the relative error. Frequency near 60 s (mean of samples 45 to 60): root mean square error in Hz. The whole trace: median over trips of the root mean square error in Hz. Each with the constants fitted on all 86, and with each trip left out of the fit in turn and the five constants refitted on the other 85.".into(),
        RESULT.trim().to_string(),
        LIMITS.trim().to_string(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "The power lost or `dt` was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was non-finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidCard" => "A constant is not this card's.",
        "InertiaNotPositive" => "The stored kinetic energy at 50 Hz is zero or less: the rate of change of frequency divides by it.",
        "DemandNegative" => "The demand is below zero.",
        "ServiceVolumeNegative" => "A contracted volume of a dynamic service is below zero.",
        "DeliveryTimeNotPositive" => "A time to full delivery is zero or less: the service's lag divides by it.",
        "LossLargerThanDemand" => "The power lost is larger in size than the demand: the area cannot lose more infeed, or more demand, than it carries.",
        "NegativeTime" => "The clock is below zero.",
        "LowFrequencyDemandDisconnection" => "The frequency, incoming or landing, is at or below 48.8 Hz, where low-frequency demand disconnection begins.",
        "FrequencyAboveCard" => "The frequency, incoming or landing, is above 50.5 Hz, the upper statutory limit.",
        "SpontaneousGeneration" => "The trial step created kinetic energy from nothing.",
        "FirstLawBreach" => "The trial step destroyed kinetic energy.",
        _ => return None,
    })
}
