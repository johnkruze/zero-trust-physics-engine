#![no_implicit_prelude]

//! The InSight entry capsule on its entry at Mars of 26 November 2018, from
//! the top of the atmosphere down toward parachute deployment.
//!
//! A 605.648 kg blunt 70° cone, 2.65 m across, flying nose first, unpowered
//! and not spun. The card runs from a radius of 3522.2 km, which is 126.5 km
//! above the datum, down to 5.7 km altitude or Mach 1.46, whichever comes
//! first, and is refused below either as `PastComparedFlight`. The flight
//! record ends where the flown capsule opened its parachute, 209.27 s after
//! the card's entry state, 6164.6 m above the datum at 352.1 m/s; there is
//! no measurement of a bare capsule below it. The floor is the lowest state
//! the compared run with the flight's lift reached in its comparison, at the
//! flight's deployment time and at the two deployment crossings scored
//! against it, 5793 m and Mach 1.470, rounded outward. Altitude is height
//! above a sphere of radius 3 395 714 m, the mean areoid radius under the
//! seven orbiter soundings.
//!
//! Only the entry from the locked entry state was compared with the flight.
//! The card takes any start, but answers only inside the corridor its
//! compared runs flew: runs from the locked entry state at 0.1 s with any
//! lift over drag from −0.08 to 0.08 held through the run, or with −0.031 or
//! 0 to 150 s and any lift in that span after. At each altitude the corridor
//! is the lowest and highest speed through the air and the lowest and
//! highest sine of the path angle those runs had within 500 m of it,
//! widened by 1 m/s and by 0.001, at knots every 500 m from 5.5 km to
//! 126.5 km, read by straight lines between knots. A state outside it,
//! incoming or landing, is refused as `OffComparedCorridor`. Every state of
//! those runs is inside it, so a restart from a state they return lands. A
//! run whose lift changes in ways not compared can leave it and is refused
//! where it does. The limit depends only on the state, not on the clock or
//! on the drag loss booked.
//!
//! Where the locked numbers come from:
//!
//! - Karatekin and Van Hove, "InSight trajectory and atmospheric
//!   reconstruction based on IMU flight data", the note of the NASA Planetary
//!   Data System bundle `insight_edl` (Royal Observatory of Belgium): the
//!   entry position and velocity, which are the navigation solution OD133,
//!   and the capsule's diameter. The same bundle's JPL ancillary file: the
//!   entry mass.
//! - Edquist, Desai, Schoenenberger, "Aerodynamics for the Mars Phoenix Entry
//!   Capsule", AIAA 2008-7219, NTRS 20080034648: the axial-force coefficient
//!   at zero angle of attack, read from the graph of Figure 9a, and the limit
//!   on lift. InSight flew the Phoenix aeroshell and its database.
//! - Korzun and coauthors, "Aerodynamic Performance of the 2018 InSight Mars
//!   Lander", NTRS 20200002812, Table 2: which drag table is read where.
//! - Mars Climate Sounder derived records, NASA Planetary Data System volume
//!   MROM_2147: temperature against altitude, the mean of seven soundings
//!   within 5° of the landing target, 0.9 to 7.0 sols before the entry.
//! - Karlgaard and coauthors, "Mars InSight Entry, Descent, and Landing
//!   Trajectory and Atmosphere Reconstruction", NTRS 20200003204: the surface
//!   pressure from the lander's own barometer, 733.724 Pa, and the radius of
//!   the landing site, 3 393 079.5 m. The air's pressure is the hydrostatic
//!   law carried up from that measurement through those temperatures.
//! - NASA Mars Fact Sheet (19 May 2025): the gravitational parameter, the
//!   rotation period, the mean molecular weight of the air.
//!
//! No number in the card comes from the capsule's accelerometer. The density
//! the reconstructions derive from the accelerometer is not used.
//!
//! The capsule was built to fly at zero angle of attack. In flight it trimmed
//! at 2° to 4° and its lift pointed toward Mars and to one side; the same
//! aeroshell on Phoenix trimmed with its lift pointing away from Mars. The
//! 2008 database states a trim lift as large as 0.068 of drag at three
//! standard deviations of pitching moment, and 0.08 with the centre of
//! gravity 5 mm off the axis. So lift is not a constant of this card. It is
//! the caller's load: lift over drag in the flight plane, positive away from
//! Mars, from −0.08 to 0.08. With a load of zero the card is a capsule with
//! no lift.
//!
//! The card has no attitude, no sideways lift, no wind and no loss of mass.
//! Its drag is the zero-angle value, with no rise in thin air above about
//! 65 km. Its air is the mean of soundings one to seven sols old, not the air
//! of that afternoon: between 36 km and 65 km it is 8 to 13 percent denser
//! than the density reconstructed from the flight.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, altitude, ground distance, Mach number, air
//! speed, radial speed, dynamic pressure, load, and the conservation residual
//! into [`LastStateFrame64`]. The 16-byte frame proof is not touched.
//!
//! Air density and speed of sound are locked tables against altitude: 631
//! knots, one every 200 m from 4 km to 130 km, read by straight lines between
//! knots. The card reads them up to its ceiling, 128 km; the knots above
//! 128.2 km are carried as copied and are never read. The file is
//! `mars_entry_insight/tables/air_for_the_card.csv` in the research folder,
//! written by `analysis/08_air_for_the_card.py`, SHA-256
//! `3c9c134b1a499a9df695831228336f56ad45dfc5e0c94778546b86fc5e266021`. Above 84.5 km,
//! the top of the soundings, the temperature is held at its 84.5 km value and
//! the pressure follows the hydrostatic law; the drag there is under a
//! twentieth of an Earth g.
//!
//! Drag is two tables from the database's figure, straight lines between
//! entries: by air speed where the Mach number is 8.8 or more, by Mach number
//! where it is 6.3 or less, and between those a straight line in Mach number
//! from the Mach table's top entry to the value read by speed. One assumption
//! reaches past a table. The speed table starts at 1.8 km/s. Between Mach 6.3
//! and Mach 8.8 the capsule is slower than that, and the card holds the speed
//! table's first entry, 1.6268, while the read hands over to the Mach table.
//! Above 5.85 km/s, the top of the speed table, there is no drag and the step
//! is refused.
//!
//! The step is classical RK4 on five numbers. Its arithmetic is `+ − × ÷` and
//! `sqrt`.
//!
//! ```text
//! h   = r − R                     altitude
//! u_r = v_r                       velocity through the air, radial
//! u_t = v_t − Ω r                 velocity through the air, transverse
//! u   = sqrt(u_r² + u_t²)
//! M   = u / c(h)
//! a   = ½ ρ(h) u² C(u, M) A / m   drag acceleration
//! l                               the load: lift over drag, positive away from Mars
//!
//! d r/dt   = v_r
//! d v_r/dt = v_t²/r − GM/r² − a u_r/u + l a u_t/u
//! d v_t/dt = −v_r v_t/r − a u_t/u − l a u_r/u
//! d θ/dt   = v_t/r
//! d Q/dt   = a u                  energy per kilogram removed by drag
//!
//! J = ½ (v_r² + v_t²) − GM/r − Ω r v_t
//! residual = |J − J_anchor + Q| / ENERGY_SCALE_J_KG
//! ```
//!
//! `r`, `v_r` and `v_t` are in a frame that does not turn with Mars. The air
//! turns with Mars at rate `Ω` in the flight plane. `J` is the energy per
//! kilogram as seen from the frame that turns with the air. Without air it is
//! constant. With air only drag changes it, by exactly `−a u`: lift is at
//! right angles to the velocity through the air and does no work there.
//!
//! The ledger shows that the drag loss booked is the work of the force that
//! was applied. It catches a force that does not match its booking, and a
//! lift that is not at right angles to the path. It is blind to a wrong air
//! table used consistently, to a wrong mass, and to a lift of the wrong size:
//! lift does no work, so every lift closes the ledger. Accuracy rests on the
//! step ceiling, on the reference-state tests and on the comparison with the
//! flight.
//!
//! [`CapsuleInsight::gate_dt_s`] is 0.1 s, and [`CapsuleInsight::floor_dt_s`]
//! is that divided by 2^20. The public step applies neither. A step taken
//! with a lift that is not zero sets [`FLAG_LIFT_ACTING`], which does not
//! refuse the step.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Entry mass. JPL ancillary file of the PDS bundle `insight_edl`. Held fixed.
pub const CAPSULE_MASS_KG: f64 = 605.648;
/// `π × (2.65 m)² / 4`, to nine significant digits: the area the database's
/// coefficients were made with.
pub const REFERENCE_AREA_M2: f64 = 5.515458602;
/// Distance from the centre of Mars at the entry state: the length of the
/// position printed in the PDS reconstruction note.
pub const ENTRY_RADIUS_M: f64 = 3522199.9999655997;
/// The printed velocity along the printed position. Negative going down.
pub const ENTRY_RADIAL_SPEED_M_S: f64 = -1202.1585438056352;
/// The rest of the printed velocity.
pub const ENTRY_TRANSVERSE_SPEED_M_S: f64 = 5633.705477158609;
/// Radius of the sphere altitude is measured from: the mean areoid radius
/// printed in the seven orbiter soundings.
pub const DATUM_RADIUS_M: f64 = 3_395_714.0;
/// NASA Mars Fact Sheet: 0.042828 × 10⁶ km³/s².
pub const MARS_GRAVITATIONAL_PARAMETER_M3_S2: f64 = 4.2828e13;
/// `2π / (24.6229 h)` times the part of Mars's rotation that lies along the
/// normal of the flight plane at the entry state: the rotation of the air
/// seen in the flight plane.
pub const AIR_ROTATION_RATE_RAD_S: f64 = 6.892326518830962e-05;
/// One Earth g, exact by definition. The unit of the reported load.
pub const STANDARD_GRAVITY_M_S2: f64 = 9.80665;

/// Top of the card, above the entry altitude of 126 486 m.
pub const ALTITUDE_CEILING_M: f64 = 128_000.0;
/// Bottom of the air tables. The card itself stops higher, at
/// [`COMPARED_ALTITUDE_FLOOR_M`].
pub const ALTITUDE_FLOOR_M: f64 = 4_000.0;
/// Bottom of the drag table read by Mach number. The card itself stops
/// higher, at [`COMPARED_MACH_FLOOR`].
pub const MACH_FLOOR: f64 = 1.0;
/// Bottom of the card: the lowest altitude the compared run with the
/// flight's lift reached in its comparison, 5793 m, rounded outward. The
/// flight record ends at 6164.6 m.
pub const COMPARED_ALTITUDE_FLOOR_M: f64 = 5_700.0;
/// Bottom of the card in Mach number: the lowest the same run reached in
/// its comparison, 1.470, rounded outward. The flight's speed at parachute
/// deployment, 352.1 m/s, is Mach 1.473 in the card's air.
pub const COMPARED_MACH_FLOOR: f64 = 1.46;
/// Top of the card in air speed: the top of the drag table read by speed.
pub const AIR_SPEED_CEILING_M_S: f64 = 5_850.0;
/// Largest lift over drag the card takes, either way: the trim value the
/// 2008 database gives for a centre of gravity 5 mm off the axis.
pub const LIFT_OVER_DRAG_LIMIT: f64 = 0.08;

/// Distance between knots of the two air tables.
pub const TABLE_KNOT_SPACING_M: f64 = 200.0;
/// Knots in each air table: 4 km to 130 km every 200 m.
pub const KNOTS: usize = 631;

/// Air density at each knot, from 4 km up, as decimal literals.
pub const AIR_DENSITY_KG_M3: [f64; KNOTS] = [
    0.009641975984,
    0.009497699022,
    0.009355339341,
    0.009214870597,
    0.009076270949,
    0.008939522624,
    0.008804523897,
    0.008671336869,
    0.008539924417,
    0.008410266472,
    0.008282358269,
    0.008156068262,
    0.008031491667,
    0.00790831965,
    0.007786551158,
    0.007666460439,
    0.007547832161,
    0.007430849703,
    0.007315369405,
    0.00720137956,
    0.007088986301,
    0.006977475417,
    0.006867553141,
    0.006759088452,
    0.006652068874,
    0.006546587569,
    0.006442270035,
    0.006339467602,
    0.006237878724,
    0.006137503419,
    0.006038609839,
    0.00594102741,
    0.005844894174,
    0.005750037789,
    0.005656449673,
    0.005564266601,
    0.005473115292,
    0.005383347485,
    0.005294897285,
    0.005207748929,
    0.005121931095,
    0.005037235919,
    0.004953843145,
    0.004871647574,
    0.004790637213,
    0.004710882552,
    0.00463234914,
    0.004555036332,
    0.00447892945,
    0.0044040109,
    0.00433026053,
    0.004257580087,
    0.00418603823,
    0.004115648268,
    0.004046391638,
    0.00397822165,
    0.003911227169,
    0.003845282712,
    0.003780364415,
    0.003716457869,
    0.003653557037,
    0.003591801281,
    0.003531014464,
    0.003471229657,
    0.003412429508,
    0.003354552184,
    0.003297591894,
    0.003241527529,
    0.003186433209,
    0.00313228964,
    0.003078995128,
    0.003026563086,
    0.002974954551,
    0.002924173542,
    0.002874207169,
    0.002825027646,
    0.002776750685,
    0.002729231674,
    0.002682455558,
    0.002636411929,
    0.002591094494,
    0.002546522653,
    0.002502654731,
    0.002459489551,
    0.002417016431,
    0.002375216331,
    0.0023340662,
    0.002293570449,
    0.002253716459,
    0.002214495104,
    0.002175900345,
    0.002137892498,
    0.002100494668,
    0.002063685115,
    0.002027456002,
    0.001991811719,
    0.001956715849,
    0.001922189102,
    0.001888212567,
    0.001854778726,
    0.001821890077,
    0.001789493743,
    0.001757628758,
    0.0017262738,
    0.001695422027,
    0.001665079307,
    0.001635228627,
    0.001605872174,
    0.001576990645,
    0.001548577557,
    0.001520637669,
    0.001493157317,
    0.001466136115,
    0.001439576219,
    0.001413469943,
    0.001387801006,
    0.001362560708,
    0.001337744489,
    0.001313359184,
    0.001289397274,
    0.001265838696,
    0.001242693759,
    0.001219938905,
    0.001197574387,
    0.001175593762,
    0.001153984751,
    0.001132810679,
    0.001111993092,
    0.001091532866,
    0.001071424097,
    0.001051654972,
    0.001032271096,
    0.001013213755,
    0.0009944968284,
    0.0009761138753,
    0.0009580407228,
    0.0009402945031,
    0.0009228475262,
    0.0009057219299,
    0.0008889110639,
    0.0008723831622,
    0.0008561507134,
    0.0008401918194,
    0.0008245112982,
    0.0008091043424,
    0.0007939577564,
    0.0007790843621,
    0.0007644625968,
    0.0007500855285,
    0.0007359496579,
    0.0007220544823,
    0.000708863735,
    0.0006958813182,
    0.0006832354202,
    0.0006709131129,
    0.0006587776427,
    0.0006463774994,
    0.0006341834001,
    0.0006220654875,
    0.0006100313361,
    0.0005982080493,
    0.0005865586458,
    0.0005751154275,
    0.0005638844588,
    0.0005528617034,
    0.00054203438,
    0.0005314071053,
    0.0005209686386,
    0.0005107043081,
    0.0005006121459,
    0.0004907011818,
    0.0004810031581,
    0.0004714785195,
    0.0004621058507,
    0.0004528840276,
    0.0004438294663,
    0.0004349478051,
    0.0004262274922,
    0.0004176532761,
    0.0004092236644,
    0.0004009490366,
    0.0003927696053,
    0.0003847434398,
    0.0003768547826,
    0.0003691022322,
    0.000361496732,
    0.0003539548639,
    0.000346559888,
    0.0003392834763,
    0.0003321253833,
    0.0003251093954,
    0.0003181602796,
    0.0003113525691,
    0.0003046514616,
    0.000298057327,
    0.00029160048,
    0.0002852217833,
    0.0002789785747,
    0.0002728418166,
    0.000266811341,
    0.0002609114454,
    0.0002551002738,
    0.0002494167859,
    0.0002438417657,
    0.0002383741479,
    0.000233028198,
    0.0002277858033,
    0.0002226607973,
    0.0002176397303,
    0.0002127211383,
    0.0002079136633,
    0.0002032172376,
    0.0001986268013,
    0.0001941335343,
    0.0001897357526,
    0.0001854377882,
    0.0001812459117,
    0.0001771487851,
    0.00017314211,
    0.000169224012,
    0.0001653946641,
    0.0001616563706,
    0.0001580025558,
    0.0001544343385,
    0.0001509495465,
    0.0001475432418,
    0.0001442109521,
    0.0001409538433,
    0.0001377721063,
    0.0001346638941,
    0.0001316256482,
    0.0001286528524,
    0.0001257471141,
    0.0001229055669,
    0.0001201268714,
    0.0001174109784,
    0.0001147540621,
    0.0001121573052,
    0.000109616394,
    0.0001071302962,
    0.0001047007456,
    0.0001023276617,
    0.0001000084927,
    9.77397581e-05,
    9.552048761e-05,
    9.335182618e-05,
    9.12379402e-05,
    8.917200979e-05,
    8.715309304e-05,
    8.518010981e-05,
    8.325186692e-05,
    8.137559521e-05,
    7.954149655e-05,
    7.775023021e-05,
    7.600071001e-05,
    7.42903817e-05,
    7.262589782e-05,
    7.099837186e-05,
    6.940884707e-05,
    6.785634214e-05,
    6.633816949e-05,
    6.485916709e-05,
    6.341263953e-05,
    6.199916885e-05,
    6.061794003e-05,
    5.926695979e-05,
    5.794973929e-05,
    5.666120296e-05,
    5.540222815e-05,
    5.417206834e-05,
    5.296860558e-05,
    5.179477875e-05,
    5.064629718e-05,
    4.952474694e-05,
    4.842939732e-05,
    4.735755817e-05,
    4.631282559e-05,
    4.529036662e-05,
    4.429181687e-05,
    4.331652215e-05,
    4.236188904e-05,
    4.143218691e-05,
    4.052200808e-05,
    3.963204996e-05,
    3.876182257e-05,
    3.79098296e-05,
    3.707974553e-05,
    3.626691331e-05,
    3.547101549e-05,
    3.46917167e-05,
    3.392866721e-05,
    3.31839299e-05,
    3.245465181e-05,
    3.173997717e-05,
    3.103966262e-05,
    3.035398512e-05,
    2.968336863e-05,
    2.902677e-05,
    2.838327155e-05,
    2.775266461e-05,
    2.713534209e-05,
    2.653117961e-05,
    2.593976791e-05,
    2.536025009e-05,
    2.47924359e-05,
    2.423669985e-05,
    2.369272632e-05,
    2.316035328e-05,
    2.263879919e-05,
    2.212788841e-05,
    2.162795788e-05,
    2.113836278e-05,
    2.06593318e-05,
    2.019009499e-05,
    1.973049523e-05,
    1.928089307e-05,
    1.884036083e-05,
    1.840946393e-05,
    1.798744487e-05,
    1.757416268e-05,
    1.716999605e-05,
    1.677406863e-05,
    1.638692178e-05,
    1.60079459e-05,
    1.56370011e-05,
    1.527434271e-05,
    1.49192047e-05,
    1.457204074e-05,
    1.423230806e-05,
    1.389987582e-05,
    1.357495928e-05,
    1.325676938e-05,
    1.294581339e-05,
    1.264153381e-05,
    1.234381624e-05,
    1.205291782e-05,
    1.1767887e-05,
    1.148943108e-05,
    1.121685566e-05,
    1.095007382e-05,
    1.068950694e-05,
    1.043402829e-05,
    1.018455417e-05,
    9.940189737e-06,
    9.700880392e-06,
    9.467272445e-06,
    9.238175644e-06,
    9.014590707e-06,
    8.795550629e-06,
    8.581017633e-06,
    8.371726651e-06,
    8.166564e-06,
    7.966462132e-06,
    7.770580255e-06,
    7.578874956e-06,
    7.39196394e-06,
    7.208938161e-06,
    7.030525336e-06,
    6.856246915e-06,
    6.686027975e-06,
    6.520127725e-06,
    6.358022754e-06,
    6.200046449e-06,
    6.046193091e-06,
    5.896346492e-06,
    5.750299598e-06,
    5.608055042e-06,
    5.469408229e-06,
    5.334646769e-06,
    5.203637836e-06,
    5.075900972e-06,
    4.951822107e-06,
    4.830816731e-06,
    4.713241283e-06,
    4.59897314e-06,
    4.487492256e-06,
    4.3792191e-06,
    4.27356264e-06,
    4.170741459e-06,
    4.070663345e-06,
    3.972977542e-06,
    3.877825508e-06,
    3.784938959e-06,
    3.694357978e-06,
    3.606020166e-06,
    3.519777868e-06,
    3.435456906e-06,
    3.353142911e-06,
    3.272761715e-06,
    3.194269834e-06,
    3.117649522e-06,
    3.042621415e-06,
    2.969394076e-06,
    2.898577079e-06,
    2.830058805e-06,
    2.763125938e-06,
    2.696156881e-06,
    2.630815676e-06,
    2.566871921e-06,
    2.504307727e-06,
    2.443281852e-06,
    2.380246758e-06,
    2.318941244e-06,
    2.261105037e-06,
    2.206487146e-06,
    2.153194623e-06,
    2.101195159e-06,
    2.050457237e-06,
    2.000950113e-06,
    1.952643792e-06,
    1.905509018e-06,
    1.859517248e-06,
    1.814640638e-06,
    1.770852026e-06,
    1.728124913e-06,
    1.686433451e-06,
    1.645752422e-06,
    1.606057226e-06,
    1.567323863e-06,
    1.529528923e-06,
    1.492649565e-06,
    1.456663509e-06,
    1.421549018e-06,
    1.387284887e-06,
    1.353850429e-06,
    1.321225462e-06,
    1.2893903e-06,
    1.258325733e-06,
    1.228013025e-06,
    1.198433896e-06,
    1.16957051e-06,
    1.141405471e-06,
    1.113921805e-06,
    1.087102952e-06,
    1.060932757e-06,
    1.035395461e-06,
    1.010475688e-06,
    9.861584366e-07,
    9.624290726e-07,
    9.392733186e-07,
    9.166772455e-07,
    8.946272638e-07,
    8.731101157e-07,
    8.521128666e-07,
    8.316228973e-07,
    8.116278964e-07,
    7.921158524e-07,
    7.730750469e-07,
    7.544940469e-07,
    7.363616983e-07,
    7.186671184e-07,
    7.013996901e-07,
    6.845490546e-07,
    6.681051058e-07,
    6.520579835e-07,
    6.363980676e-07,
    6.211159726e-07,
    6.062025412e-07,
    5.916488392e-07,
    5.774461499e-07,
    5.635859687e-07,
    5.500599981e-07,
    5.368601423e-07,
    5.239785029e-07,
    5.114073732e-07,
    4.991392344e-07,
    4.871667502e-07,
    4.754827632e-07,
    4.640802895e-07,
    4.529525156e-07,
    4.420927933e-07,
    4.31494636e-07,
    4.211517151e-07,
    4.110578554e-07,
    4.012070322e-07,
    3.915933669e-07,
    3.822111238e-07,
    3.730547066e-07,
    3.64118655e-07,
    3.553976411e-07,
    3.468864667e-07,
    3.385800597e-07,
    3.30473471e-07,
    3.225618718e-07,
    3.148405507e-07,
    3.073049103e-07,
    2.99950465e-07,
    2.927728381e-07,
    2.857677589e-07,
    2.789310604e-07,
    2.722586769e-07,
    2.65746641e-07,
    2.593910817e-07,
    2.531882218e-07,
    2.471343758e-07,
    2.412259474e-07,
    2.354594277e-07,
    2.298313926e-07,
    2.243385012e-07,
    2.189774935e-07,
    2.137451885e-07,
    2.086384821e-07,
    2.036543458e-07,
    1.987898241e-07,
    1.940420333e-07,
    1.894081593e-07,
    1.848854564e-07,
    1.804712452e-07,
    1.761629114e-07,
    1.719579037e-07,
    1.678537328e-07,
    1.638479694e-07,
    1.599382433e-07,
    1.561222413e-07,
    1.523977065e-07,
    1.487624363e-07,
    1.452142816e-07,
    1.417511452e-07,
    1.383709806e-07,
    1.350717908e-07,
    1.31851627e-07,
    1.287085878e-07,
    1.256408174e-07,
    1.226465051e-07,
    1.197238837e-07,
    1.168712291e-07,
    1.140868586e-07,
    1.1136913e-07,
    1.087164411e-07,
    1.061272283e-07,
    1.035999656e-07,
    1.01133164e-07,
    9.872537046e-08,
    9.637516695e-08,
    9.40811697e-08,
    9.184202835e-08,
    8.965642512e-08,
    8.752307403e-08,
    8.544072013e-08,
    8.340813876e-08,
    8.142413479e-08,
    7.948754192e-08,
    7.759722198e-08,
    7.575206425e-08,
    7.39509848e-08,
    7.219292581e-08,
    7.0476855e-08,
    6.880176492e-08,
    6.716667246e-08,
    6.557061817e-08,
    6.401266572e-08,
    6.249190134e-08,
    6.100743328e-08,
    5.955839129e-08,
    5.814392604e-08,
    5.676320868e-08,
    5.541543033e-08,
    5.409980155e-08,
    5.281555195e-08,
    5.156192964e-08,
    5.033820087e-08,
    4.914364953e-08,
    4.797757674e-08,
    4.683930043e-08,
    4.572815496e-08,
    4.46434907e-08,
    4.358467363e-08,
    4.255108498e-08,
    4.154212089e-08,
    4.055719198e-08,
    3.959572308e-08,
    3.86571528e-08,
    3.774093329e-08,
    3.684652985e-08,
    3.597342062e-08,
    3.51210963e-08,
    3.428905981e-08,
    3.347682601e-08,
    3.268392144e-08,
    3.190988398e-08,
    3.115426263e-08,
    3.041661721e-08,
    2.969651811e-08,
    2.899354604e-08,
    2.830729178e-08,
    2.76373559e-08,
    2.69833486e-08,
    2.634488942e-08,
    2.5721607e-08,
    2.511313893e-08,
    2.451913148e-08,
    2.393923939e-08,
    2.33731257e-08,
    2.282046152e-08,
    2.228092586e-08,
    2.175420541e-08,
    2.123999438e-08,
    2.07379943e-08,
    2.024791387e-08,
    1.976946875e-08,
    1.930238144e-08,
    1.884638108e-08,
    1.840120327e-08,
    1.796658999e-08,
    1.754228937e-08,
    1.712805558e-08,
    1.672364867e-08,
    1.632883445e-08,
    1.594338433e-08,
    1.556707519e-08,
    1.519968925e-08,
    1.484101394e-08,
    1.449084179e-08,
    1.414897028e-08,
    1.381520175e-08,
    1.348934324e-08,
    1.317120644e-08,
    1.286060752e-08,
    1.255736707e-08,
    1.226130993e-08,
    1.197226518e-08,
    1.169006594e-08,
    1.141454935e-08,
    1.114555643e-08,
    1.088293199e-08,
    1.062652456e-08,
    1.037618631e-08,
    1.013177289e-08,
    9.893143444e-09,
    9.660160462e-09,
    9.432689721e-09,
];

/// Speed of sound at each knot, from 4 km up, as decimal literals.
pub const SOUND_SPEED_M_S: [f64; KNOTS] = [
    241.4749646,
    241.2527025,
    241.0302354,
    240.8076156,
    240.5848426,
    240.3618631,
    240.1397778,
    239.9174869,
    239.6952127,
    239.4729551,
    239.250491,
    239.0294581,
    238.8082206,
    238.5911474,
    238.37825,
    238.1651622,
    237.9549825,
    237.744617,
    237.5360776,
    237.3293692,
    237.1224806,
    236.9273027,
    236.7319638,
    236.5384094,
    236.3466437,
    236.1547223,
    235.9691965,
    235.7835246,
    235.6030488,
    235.4277808,
    235.2523822,
    235.0798597,
    234.9072105,
    234.7375649,
    234.5709292,
    234.4041751,
    234.2449443,
    234.0856052,
    233.9271902,
    233.7697011,
    233.6121058,
    233.4588262,
    233.3054458,
    233.1540475,
    233.0046352,
    232.855127,
    232.705938,
    232.5566533,
    232.4071961,
    232.2575661,
    232.1078396,
    231.9602527,
    231.8125718,
    231.6639517,
    231.5143904,
    231.3647325,
    231.2118545,
    231.0588754,
    230.9060591,
    230.7534062,
    230.6006522,
    230.4428096,
    230.2848588,
    230.1252306,
    229.9639216,
    229.8024993,
    229.6406978,
    229.4787822,
    229.3136032,
    229.1451538,
    228.9765805,
    228.806905,
    228.6371036,
    228.4665527,
    228.2952508,
    228.1238202,
    227.9469956,
    227.7700338,
    227.5931131,
    227.4162337,
    227.2392167,
    227.0607178,
    226.8820785,
    226.7028834,
    226.5231312,
    226.3432363,
    226.1638504,
    225.9843221,
    225.8048087,
    225.6253103,
    225.4456689,
    225.2675097,
    225.0892095,
    224.9114688,
    224.7342891,
    224.5569696,
    224.3811191,
    224.2051307,
    224.0296282,
    223.8546129,
    223.6794607,
    223.5069926,
    223.3343913,
    223.1625224,
    222.9913877,
    222.8201216,
    222.6493632,
    222.4784738,
    222.3082879,
    222.1388073,
    221.9691973,
    221.7999391,
    221.6305516,
    221.4603226,
    221.2892504,
    221.1180458,
    220.9468696,
    220.7755607,
    220.6029892,
    220.4291522,
    220.255178,
    220.0795873,
    219.9038565,
    219.7274066,
    219.5502357,
    219.3729218,
    219.188713,
    219.0043493,
    218.8191911,
    218.6332365,
    218.4471236,
    218.255423,
    218.063554,
    217.8694386,
    217.6730708,
    217.4765257,
    217.2772517,
    217.0777948,
    216.8749419,
    216.6686837,
    216.462229,
    216.2534371,
    216.0444436,
    215.8340695,
    215.622311,
    215.4103443,
    215.1958293,
    214.9811002,
    214.7666064,
    214.5523485,
    214.3378764,
    214.0518327,
    213.7654063,
    213.4580246,
    213.1295969,
    212.8006623,
    212.5458335,
    212.2906988,
    212.0569973,
    211.8448,
    211.6323899,
    211.4259,
    211.2192083,
    211.0105547,
    210.7999336,
    210.5891018,
    210.3765117,
    210.1637065,
    209.9530957,
    209.7446859,
    209.5360688,
    209.3196152,
    209.1029377,
    208.8902714,
    208.6816287,
    208.4727772,
    208.2616895,
    208.0503876,
    207.8420273,
    207.6366174,
    207.4310041,
    207.2404504,
    207.0497213,
    206.8624425,
    206.6786233,
    206.4946405,
    206.3343281,
    206.173891,
    206.0211664,
    205.8761712,
    205.7310739,
    205.6095948,
    205.4880439,
    205.3772916,
    205.2773553,
    205.1773703,
    205.0978431,
    205.0182851,
    204.9486093,
    204.8888258,
    204.8290248,
    204.7850752,
    204.7411162,
    204.7040914,
    204.6740046,
    204.6439134,
    204.6208264,
    204.5977368,
    204.5797653,
    204.5669134,
    204.5540607,
    204.539964,
    204.5258664,
    204.5151744,
    204.5078886,
    204.5006026,
    204.488442,
    204.4762807,
    204.4653996,
    204.4557989,
    204.4461977,
    204.4338349,
    204.4214714,
    204.4070919,
    204.3906961,
    204.3742989,
    204.3598415,
    204.345383,
    204.3295173,
    204.3122441,
    204.2949694,
    204.2800583,
    204.265146,
    204.2513656,
    204.2387171,
    204.2260678,
    204.2155842,
    204.2051002,
    204.1973799,
    204.1924239,
    204.1874677,
    204.1812911,
    204.1751142,
    204.1713034,
    204.1698587,
    204.168414,
    204.1609415,
    204.1534687,
    204.1458213,
    204.1379992,
    204.1301768,
    204.1118901,
    204.0936018,
    204.0731937,
    204.0506652,
    204.0281341,
    203.9949063,
    203.9616731,
    203.9256913,
    203.8869596,
    203.8482204,
    203.8005909,
    203.7529503,
    203.7031765,
    203.651268,
    203.5993463,
    203.5400159,
    203.4806683,
    203.4185533,
    203.3536686,
    203.2887632,
    203.2168803,
    203.144972,
    203.0686683,
    202.9879642,
    202.9072281,
    202.8174083,
    202.7275488,
    202.6328309,
    202.5332478,
    202.4336157,
    202.3223476,
    202.2110183,
    202.0968345,
    201.9797914,
    201.8626805,
    201.734385,
    201.6060078,
    201.4774858,
    201.3488185,
    201.220069,
    201.0839284,
    200.9476955,
    200.8131302,
    200.680236,
    200.5472536,
    200.411747,
    200.2761488,
    200.1427328,
    200.0115034,
    199.8801878,
    199.7482513,
    199.6162276,
    199.4865001,
    199.3590732,
    199.2315649,
    199.1042813,
    198.9769164,
    198.8518866,
    198.7291964,
    198.6064304,
    198.485587,
    198.3646699,
    198.246411,
    198.1308151,
    198.0151517,
    197.9032495,
    197.791284,
    197.6823292,
    197.5763902,
    197.4703944,
    197.3684128,
    197.2663784,
    197.1669091,
    197.0700089,
    196.9730611,
    196.8799916,
    196.7868782,
    196.6962932,
    196.6082401,
    196.5201476,
    196.4367014,
    196.3532197,
    196.2728118,
    196.1954813,
    196.1181203,
    196.0474733,
    195.9768008,
    195.910905,
    195.8497908,
    195.7886576,
    195.7368585,
    195.6850458,
    195.6407057,
    195.6038432,
    195.5669738,
    195.5414108,
    195.5158444,
    195.4996133,
    195.4927199,
    195.4858262,
    195.4908729,
    195.4959195,
    195.510018,
    195.5331664,
    195.5563121,
    195.5902715,
    195.624225,
    195.6633448,
    195.7076279,
    195.7519009,
    195.8025791,
    195.8532443,
    195.9022349,
    195.9495523,
    195.9968584,
    196.0423631,
    196.0878573,
    196.1263012,
    196.157699,
    196.1890919,
    196.2110716,
    196.2330489,
    196.2459413,
    196.2497506,
    196.2535597,
    196.2462782,
    196.2389964,
    196.225041,
    196.2044106,
    196.183778,
    196.158062,
    196.1323426,
    196.1041174,
    196.0733853,
    196.0426484,
    196.0155129,
    195.9883737,
    195.9620352,
    195.9364977,
    195.9109569,
    195.8930712,
    195.8751838,
    195.8351066,
    195.7728258,
    195.7105252,
    195.7064456,
    195.7023659,
    195.7056011,
    195.7161507,
    195.7266998,
    195.883284,
    196.0397431,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
    196.1179258,
];

/// Entries in the drag table read by air speed.
pub const DRAG_SPEED_ENTRIES: usize = 21;
/// Entries in the drag table read by Mach number.
pub const DRAG_MACH_ENTRIES: usize = 9;
/// Air speeds of the hypersonic drag table, km/s. Phoenix aerodynamics paper,
/// Figure 9a, read from the graph.
pub const DRAG_SPEED_KM_S: [f64; DRAG_SPEED_ENTRIES] = [
    1.8, 2.0, 2.2, 2.4, 2.6, 2.8, 3.0, 3.2, 3.4, 3.6, 3.8, 4.0, 4.2, 4.4, 4.6, 4.8, 5.0, 5.2, 5.4,
    5.6, 5.85,
];
/// Axial-force coefficient at those speeds, zero angle of attack.
pub const DRAG_BY_SPEED: [f64; DRAG_SPEED_ENTRIES] = [
    1.6268, 1.6352, 1.6420, 1.6488, 1.6538, 1.6623, 1.6724, 1.6859, 1.6978, 1.7096, 1.7180, 1.7248,
    1.7290, 1.7282, 1.7282, 1.7265, 1.7265, 1.7265, 1.7248, 1.7248, 1.7146,
];
/// Mach numbers of the supersonic drag table. The same figure. The card
/// reads this table from Mach 1.0 up.
pub const DRAG_MACH: [f64; DRAG_MACH_ENTRIES] = [0.9, 1.0, 1.2, 1.5, 2.0, 3.0, 4.1, 5.1, 6.3];
/// Axial-force coefficient at those Mach numbers, zero angle of attack.
pub const DRAG_BY_MACH: [f64; DRAG_MACH_ENTRIES] = [
    1.2145, 1.3007, 1.4156, 1.5423, 1.5710, 1.5778, 1.5862, 1.5964, 1.6082,
];
/// Drag is read by Mach number at and below this. InSight aerodynamics paper, Table 2.
pub const DRAG_MACH_TABLE_TOP: f64 = 6.3;
/// Drag is read by air speed at and above this Mach number. The same table.
pub const DRAG_SPEED_TABLE_MACH_FLOOR: f64 = 8.8;

/// Knots of the corridor the compared runs flew: 5.5 km to 126.5 km every 500 m.
pub const CORRIDOR_KNOTS: usize = 243;
/// Lowest knot of the corridor, under [`COMPARED_ALTITUDE_FLOOR_M`].
pub const CORRIDOR_BOTTOM_M: f64 = 5_500.0;
/// Distance between corridor knots.
pub const CORRIDOR_KNOT_SPACING_M: f64 = 500.0;
/// Top of the corridor, its last knot, just above the entry altitude of
/// 126 486 m. No compared run was higher.
pub const CORRIDOR_TOP_M: f64 = 126_500.0;
// The four corridor tables. Each knot holds the lowest or highest value that
// the compared runs had within 500 m of its altitude, rounded outward and
// widened by 1 m/s or by 0.001: runs from the locked entry state at 0.1 s,
// lift over drag held from −0.08 to 0.08 every 0.0025 and at the values the
// second scoring ran, and −0.031 or 0 to 150 s and then each of those, cut at
// the compared floor. Written by the 0.7.0 fix from the 0.6.0 kernel's runs.
/// Lowest speed through the air at each corridor knot, m/s.
pub const CORRIDOR_AIR_SPEED_LOW_M_S: [f64; CORRIDOR_KNOTS] = [
    348.39, 347.58, 346.80, 346.07, 345.37, 344.74, 344.01, 343.48, 343.48, 346.79, 355.47, 364.47,
    373.78, 383.78, 393.74, 404.43, 415.49, 426.92, 438.74, 451.42, 464.54, 478.10, 492.60, 507.58,
    523.60, 540.17, 557.31, 575.65, 595.26, 615.60, 637.38, 660.71, 685.72, 712.56, 741.40, 773.26,
    806.74, 843.82, 884.96, 930.73, 981.82, 1040.32, 1106.28, 1182.57, 1271.45, 1374.12, 1493.81,
    1630.02, 1780.71, 1938.48, 2104.63, 2272.96, 2435.17, 2594.66, 2747.76, 2896.06, 3036.78,
    3167.34, 3297.33, 3419.55, 3532.65, 3648.23, 3752.45, 3850.97, 3949.85, 4042.08, 4127.31,
    4211.83, 4288.87, 4358.44, 4431.24, 4498.23, 4559.09, 4616.95, 4673.19, 4726.70, 4775.09,
    4820.91, 4865.09, 4907.56, 4947.77, 4983.02, 5019.19, 5051.45, 5081.89, 5110.68, 5138.17,
    5162.24, 5186.84, 5210.14, 5230.60, 5250.01, 5269.68, 5286.95, 5303.33, 5318.69, 5333.27,
    5347.03, 5358.92, 5371.19, 5382.79, 5393.71, 5403.14, 5412.87, 5421.25, 5429.93, 5437.38,
    5445.10, 5451.71, 5457.97, 5463.87, 5469.96, 5475.19, 5480.12, 5484.76, 5489.13, 5493.25,
    5497.12, 5500.76, 5504.17, 5507.38, 5510.39, 5513.21, 5515.89, 5518.06, 5520.38, 5522.55,
    5524.61, 5526.24, 5528.00, 5529.67, 5530.98, 5532.41, 5533.55, 5534.77, 5535.91, 5536.79,
    5537.76, 5538.50, 5539.19, 5539.94, 5540.50, 5541.12, 5541.58, 5541.99, 5542.44, 5542.76,
    5543.06, 5543.36, 5543.58, 5543.77, 5543.93, 5544.08, 5544.19, 5544.26, 5544.32, 5544.35,
    5544.33, 5544.29, 5544.23, 5544.15, 5544.06, 5543.96, 5543.84, 5543.71, 5543.57, 5543.41,
    5543.25, 5543.07, 5542.89, 5542.73, 5542.53, 5542.32, 5542.11, 5541.88, 5541.65, 5541.41,
    5541.22, 5540.97, 5540.72, 5540.46, 5540.19, 5539.97, 5539.70, 5539.42, 5539.20, 5538.91,
    5538.62, 5538.33, 5538.09, 5537.79, 5537.55, 5537.24, 5536.93, 5536.69, 5536.37, 5536.12,
    5535.80, 5535.48, 5535.22, 5534.90, 5534.64, 5534.31, 5534.04, 5533.71, 5533.45, 5533.11,
    5532.84, 5532.50, 5532.23, 5531.89, 5531.61, 5531.34, 5530.99, 5530.72, 5530.37, 5530.09,
    5529.81, 5529.46, 5529.18, 5528.89, 5528.54, 5528.25, 5527.97, 5527.61, 5527.33, 5527.04,
    5526.68, 5526.39, 5526.10, 5525.81, 5525.44, 5525.15, 5524.86, 5524.57, 5524.20, 5523.91,
    5523.61, 5523.32, 5523.02, 5522.72, 5522.35, 5522.35,
];
/// Highest speed through the air at each corridor knot, m/s.
pub const CORRIDOR_AIR_SPEED_HIGH_M_S: [f64; CORRIDOR_KNOTS] = [
    440.26, 460.90, 484.27, 508.33, 534.22, 562.09, 590.61, 621.22, 652.38, 685.71, 721.38, 759.54,
    798.16, 836.98, 880.73, 924.71, 968.61, 1018.07, 1067.44, 1119.68, 1171.41, 1225.93, 1283.38,
    1339.74, 1403.20, 1465.39, 1525.78, 1593.72, 1659.58, 1728.15, 1799.49, 1867.86, 1944.77,
    2018.42, 2094.52, 2166.37, 2247.00, 2322.92, 2400.69, 2480.24, 2561.45, 2644.22, 2720.78,
    2806.34, 2885.25, 2965.13, 3045.88, 3127.44, 3201.42, 3284.14, 3367.26, 3442.27, 3517.36,
    3592.42, 3667.29, 3741.83, 3815.89, 3889.39, 3962.29, 4026.46, 4097.68, 4159.99, 4223.52,
    4288.85, 4347.59, 4405.03, 4461.09, 4515.73, 4568.89, 4620.50, 4666.52, 4713.89, 4759.79,
    4800.98, 4843.70, 4881.62, 4919.97, 4954.78, 4987.55, 5021.58, 5051.50, 5080.05, 5107.26,
    5132.92, 5157.39, 5180.69, 5202.87, 5223.81, 5243.72, 5260.97, 5278.85, 5295.74, 5310.44,
    5325.60, 5339.93, 5352.33, 5365.12, 5376.19, 5387.63, 5397.51, 5406.84, 5416.47, 5424.79,
    5432.65, 5440.76, 5447.73, 5454.33, 5460.55, 5466.42, 5471.96, 5477.19, 5482.12, 5486.76,
    5491.13, 5495.25, 5499.12, 5502.76, 5506.18, 5509.39, 5512.40, 5514.90, 5517.58, 5520.07,
    5522.39, 5524.28, 5526.35, 5528.25, 5529.77, 5531.45, 5532.98, 5534.21, 5535.56, 5536.60,
    5537.76, 5538.65, 5539.63, 5540.38, 5541.20, 5541.83, 5542.41, 5543.03, 5543.50, 5544.00,
    5544.38, 5544.71, 5545.01, 5545.32, 5545.55, 5545.74, 5545.91, 5546.07, 5546.18, 5546.26,
    5546.32, 5546.36, 5546.38, 5546.38, 5546.38, 5546.37, 5546.34, 5546.29, 5546.23, 5546.15,
    5546.05, 5545.95, 5545.83, 5545.69, 5545.55, 5545.39, 5545.22, 5545.05, 5544.86, 5544.70,
    5544.50, 5544.29, 5544.07, 5543.85, 5543.62, 5543.38, 5543.18, 5542.93, 5542.67, 5542.41,
    5542.15, 5541.93, 5541.66, 5541.38, 5541.15, 5540.86, 5540.57, 5540.28, 5540.04, 5539.74,
    5539.50, 5539.19, 5538.88, 5538.63, 5538.32, 5538.07, 5537.75, 5537.43, 5537.17, 5536.84,
    5536.58, 5536.25, 5535.99, 5535.66, 5535.39, 5535.05, 5534.78, 5534.44, 5534.17, 5533.83,
    5533.56, 5533.28, 5532.93, 5532.66, 5532.31, 5532.03, 5531.75, 5531.40, 5531.12, 5530.83,
    5530.48, 5530.19, 5529.91, 5529.55, 5529.26, 5528.98, 5528.62, 5528.33, 5528.04, 5527.75,
    5527.38, 5527.09, 5526.80, 5526.50, 5526.14, 5525.84, 5525.55, 5525.25, 5524.96, 5524.66,
];
/// Lowest sine of the path angle through the air, radial air speed over air speed, at each corridor knot.
pub const CORRIDOR_PATH_SINE_LOW: [f64; CORRIDOR_KNOTS] = [
    -0.60566, -0.60566, -0.59706, -0.57826, -0.56027, -0.54156, -0.54682, -0.56257, -0.56257,
    -0.55645, -0.54058, -0.52473, -0.50893, -0.49262, -0.47701, -0.46095, -0.44505, -0.42934,
    -0.41384, -0.39802, -0.38246, -0.36719, -0.35170, -0.33654, -0.32123, -0.30630, -0.29177,
    -0.27719, -0.26260, -0.24848, -0.23444, -0.22564, -0.22069, -0.21625, -0.21155, -0.20732,
    -0.20321, -0.19954, -0.19565, -0.19219, -0.18883, -0.18557, -0.18242, -0.17937, -0.17669,
    -0.17385, -0.17135, -0.16895, -0.16663, -0.16441, -0.16248, -0.16043, -0.15847, -0.15678,
    -0.15517, -0.15363, -0.15217, -0.15079, -0.14949, -0.14826, -0.14710, -0.14614, -0.14513,
    -0.14430, -0.14353, -0.14274, -0.14210, -0.14152, -0.14100, -0.14054, -0.14013, -0.13979,
    -0.13953, -0.13928, -0.13909, -0.13896, -0.13887, -0.13885, -0.13892, -0.13904, -0.13918,
    -0.13935, -0.13955, -0.13978, -0.14004, -0.14032, -0.14063, -0.14096, -0.14132, -0.14165,
    -0.14204, -0.14246, -0.14283, -0.14329, -0.14376, -0.14418, -0.14468, -0.14512, -0.14565,
    -0.14611, -0.14658, -0.14715, -0.14764, -0.14814, -0.14873, -0.14925, -0.14977, -0.15030,
    -0.15084, -0.15138, -0.15193, -0.15248, -0.15304, -0.15361, -0.15417, -0.15474, -0.15532,
    -0.15590, -0.15648, -0.15707, -0.15756, -0.15816, -0.15875, -0.15935, -0.15985, -0.16046,
    -0.16106, -0.16157, -0.16218, -0.16279, -0.16330, -0.16392, -0.16443, -0.16505, -0.16557,
    -0.16619, -0.16671, -0.16733, -0.16785, -0.16837, -0.16900, -0.16952, -0.17015, -0.17067,
    -0.17120, -0.17172, -0.17235, -0.17288, -0.17340, -0.17393, -0.17456, -0.17509, -0.17562,
    -0.17614, -0.17667, -0.17720, -0.17783, -0.17836, -0.17889, -0.17942, -0.17994, -0.18047,
    -0.18100, -0.18153, -0.18206, -0.18259, -0.18312, -0.18364, -0.18417, -0.18470, -0.18512,
    -0.18565, -0.18618, -0.18671, -0.18724, -0.18777, -0.18829, -0.18872, -0.18924, -0.18977,
    -0.19030, -0.19083, -0.19125, -0.19178, -0.19230, -0.19273, -0.19325, -0.19378, -0.19431,
    -0.19473, -0.19525, -0.19568, -0.19620, -0.19673, -0.19715, -0.19767, -0.19809, -0.19862,
    -0.19914, -0.19957, -0.20009, -0.20051, -0.20103, -0.20145, -0.20198, -0.20240, -0.20292,
    -0.20334, -0.20386, -0.20428, -0.20481, -0.20523, -0.20564, -0.20617, -0.20658, -0.20711,
    -0.20752, -0.20794, -0.20846, -0.20888, -0.20930, -0.20982, -0.21024, -0.21065, -0.21118,
    -0.21159, -0.21201, -0.21253, -0.21295, -0.21336, -0.21378, -0.21430, -0.21471, -0.21513,
    -0.21554, -0.21606, -0.21648, -0.21689, -0.21731, -0.21772, -0.21814, -0.21866, -0.21866,
];
/// Highest sine of the path angle through the air at each corridor knot.
pub const CORRIDOR_PATH_SINE_HIGH: [f64; CORRIDOR_KNOTS] = [
    -0.51457, -0.49808, -0.47291, -0.44541, -0.42387, -0.40369, -0.38305, -0.36395, -0.34583,
    -0.32874, -0.31273, -0.29828, -0.28447, -0.27219, -0.26134, -0.25150, -0.24296, -0.23560,
    -0.22911, -0.22016, -0.20921, -0.19898, -0.19006, -0.18237, -0.17559, -0.16989, -0.16518,
    -0.16135, -0.15824, -0.15604, -0.14827, -0.14034, -0.13249, -0.12471, -0.11734, -0.11003,
    -0.10293, -0.09611, -0.08934, -0.08281, -0.07649, -0.07035, -0.06429, -0.05843, -0.05299,
    -0.04925, -0.04736, -0.04706, -0.04706, -0.04716, -0.04832, -0.05043, -0.05310, -0.05613,
    -0.05932, -0.06260, -0.06583, -0.06892, -0.07205, -0.07505, -0.07785, -0.08074, -0.08338,
    -0.08589, -0.08844, -0.09084, -0.09308, -0.09533, -0.09740, -0.09930, -0.10137, -0.10325,
    -0.10495, -0.10664, -0.10832, -0.10997, -0.11145, -0.11291, -0.11436, -0.11578, -0.11719,
    -0.11843, -0.11980, -0.12101, -0.12220, -0.12338, -0.12454, -0.12555, -0.12668, -0.12781,
    -0.12878, -0.12974, -0.13083, -0.13177, -0.13271, -0.13363, -0.13455, -0.13546, -0.13623,
    -0.13713, -0.13801, -0.13889, -0.13964, -0.14051, -0.14125, -0.14211, -0.14284, -0.14368,
    -0.14440, -0.14512, -0.14584, -0.14667, -0.14737, -0.14808, -0.14878, -0.14948, -0.15018,
    -0.15087, -0.15156, -0.15225, -0.15293, -0.15362, -0.15430, -0.15498, -0.15555, -0.15622,
    -0.15690, -0.15757, -0.15813, -0.15880, -0.15946, -0.16002, -0.16068, -0.16124, -0.16190,
    -0.16256, -0.16311, -0.16377, -0.16432, -0.16487, -0.16552, -0.16607, -0.16672, -0.16727,
    -0.16781, -0.16846, -0.16900, -0.16955, -0.17020, -0.17074, -0.17128, -0.17182, -0.17246,
    -0.17300, -0.17354, -0.17408, -0.17461, -0.17515, -0.17569, -0.17633, -0.17687, -0.17740,
    -0.17794, -0.17847, -0.17901, -0.17954, -0.18008, -0.18061, -0.18114, -0.18168, -0.18221,
    -0.18274, -0.18317, -0.18370, -0.18423, -0.18476, -0.18529, -0.18582, -0.18635, -0.18678,
    -0.18731, -0.18784, -0.18837, -0.18890, -0.18932, -0.18985, -0.19038, -0.19080, -0.19133,
    -0.19186, -0.19238, -0.19281, -0.19333, -0.19376, -0.19428, -0.19481, -0.19523, -0.19576,
    -0.19618, -0.19671, -0.19723, -0.19765, -0.19818, -0.19860, -0.19912, -0.19954, -0.20007,
    -0.20049, -0.20101, -0.20143, -0.20195, -0.20237, -0.20290, -0.20332, -0.20373, -0.20426,
    -0.20468, -0.20520, -0.20562, -0.20603, -0.20656, -0.20697, -0.20739, -0.20791, -0.20833,
    -0.20875, -0.20927, -0.20969, -0.21010, -0.21062, -0.21104, -0.21145, -0.21187, -0.21239,
    -0.21281, -0.21322, -0.21364, -0.21416, -0.21457, -0.21499, -0.21540, -0.21582, -0.21623,
];


/// Half the square of the entry air speed, 5523.36 m/s, to nine significant
/// digits. Scale of the residual.
pub const ENERGY_SCALE_J_KG: f64 = 15_253_730.93;
/// `|J − J_anchor + Q| / ENERGY_SCALE_J_KG` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;
/// Largest step the card stands behind. At the entry state the capsule falls
/// 120 m in one such step, under one knot interval of the air tables, and
/// over the whole entry the residual stays near a thousandth of its limit.
pub const GATE_DT_S: f64 = 0.1;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// The step that wrote this frame was taken with a lift that is not zero.
pub const FLAG_LIFT_ACTING: u64 = 1 << 0;

/// Why a step did not advance the capsule or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` or the lift over drag was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant or table entry is not this capsule.
    InvalidCapsule,
    /// The lift over drag is above 0.08 or below −0.08.
    LiftOutsideCard,
    /// Altitude, incoming, at a stage of the step, or landing, is above 128 km or below 4 km.
    AltitudeOutsideCard,
    /// Mach number, incoming, at a stage of the step, or landing, is below 1.0.
    MachOutsideCard,
    /// Air speed, incoming, at a stage of the step, or landing, is above 5850 m/s.
    AirSpeedOutsideCard,
    /// Altitude, incoming, at a stage of the step, or landing, is below 5700 m, or
    /// Mach number is below 1.46: past the lowest state the compared run reached.
    PastComparedFlight,
    /// The state, incoming or landing, is off the corridor the compared runs from
    /// the entry state flew: above 126.5 km, or its air speed or path angle is outside
    /// the corridor at its altitude.
    OffComparedCorridor,
    /// The trial step gained energy that drag did not account for.
    SpontaneousGeneration,
    /// The trial step lost more energy than drag accounts for.
    FirstLawBreach,
}

/// What the caller states about the flight: the capsule's lift.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CapsuleLoad {
    /// Lift over drag in the flight plane, positive away from Mars. Zero is a
    /// capsule with no lift.
    pub lift_over_drag: f64,
}

/// The locked capsule, its air, and its Mars. The tables are recorded copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapsuleInsight {
    pub capsule_mass_kg: f64,
    pub reference_area_m2: f64,
    pub entry_radius_m: f64,
    pub entry_radial_speed_m_s: f64,
    pub entry_transverse_speed_m_s: f64,
    pub datum_radius_m: f64,
    pub mars_gravitational_parameter_m3_s2: f64,
    pub air_rotation_rate_rad_s: f64,
    pub standard_gravity_m_s2: f64,
    pub altitude_ceiling_m: f64,
    pub altitude_floor_m: f64,
    pub mach_floor: f64,
    pub air_speed_ceiling_m_s: f64,
    pub lift_over_drag_limit: f64,
    pub table_knot_spacing_m: f64,
    pub air_density_kg_m3: [f64; KNOTS],
    pub sound_speed_m_s: [f64; KNOTS],
    pub drag_speed_km_s: [f64; DRAG_SPEED_ENTRIES],
    pub drag_by_speed: [f64; DRAG_SPEED_ENTRIES],
    pub drag_mach: [f64; DRAG_MACH_ENTRIES],
    pub drag_by_mach: [f64; DRAG_MACH_ENTRIES],
    pub drag_mach_table_top: f64,
    pub drag_speed_table_mach_floor: f64,
    pub compared_altitude_floor_m: f64,
    pub compared_mach_floor: f64,
    pub corridor_bottom_m: f64,
    pub corridor_knot_spacing_m: f64,
    pub corridor_air_speed_low_m_s: [f64; CORRIDOR_KNOTS],
    pub corridor_air_speed_high_m_s: [f64; CORRIDOR_KNOTS],
    pub corridor_path_sine_low: [f64; CORRIDOR_KNOTS],
    pub corridor_path_sine_high: [f64; CORRIDOR_KNOTS],
    pub energy_scale_j_kg: f64,
    pub residual_conservation_limit: f64,
}

impl CapsuleInsight {
    /// The InSight entry capsule. The constructor takes no arguments.
    #[must_use]
    pub fn entry_capsule() -> Self {
        let capsule = Self {
            capsule_mass_kg: CAPSULE_MASS_KG,
            reference_area_m2: REFERENCE_AREA_M2,
            entry_radius_m: ENTRY_RADIUS_M,
            entry_radial_speed_m_s: ENTRY_RADIAL_SPEED_M_S,
            entry_transverse_speed_m_s: ENTRY_TRANSVERSE_SPEED_M_S,
            datum_radius_m: DATUM_RADIUS_M,
            mars_gravitational_parameter_m3_s2: MARS_GRAVITATIONAL_PARAMETER_M3_S2,
            air_rotation_rate_rad_s: AIR_ROTATION_RATE_RAD_S,
            standard_gravity_m_s2: STANDARD_GRAVITY_M_S2,
            altitude_ceiling_m: ALTITUDE_CEILING_M,
            altitude_floor_m: ALTITUDE_FLOOR_M,
            mach_floor: MACH_FLOOR,
            air_speed_ceiling_m_s: AIR_SPEED_CEILING_M_S,
            lift_over_drag_limit: LIFT_OVER_DRAG_LIMIT,
            table_knot_spacing_m: TABLE_KNOT_SPACING_M,
            air_density_kg_m3: AIR_DENSITY_KG_M3,
            sound_speed_m_s: SOUND_SPEED_M_S,
            drag_speed_km_s: DRAG_SPEED_KM_S,
            drag_by_speed: DRAG_BY_SPEED,
            drag_mach: DRAG_MACH,
            drag_by_mach: DRAG_BY_MACH,
            drag_mach_table_top: DRAG_MACH_TABLE_TOP,
            drag_speed_table_mach_floor: DRAG_SPEED_TABLE_MACH_FLOOR,
            compared_altitude_floor_m: COMPARED_ALTITUDE_FLOOR_M,
            compared_mach_floor: COMPARED_MACH_FLOOR,
            corridor_bottom_m: CORRIDOR_BOTTOM_M,
            corridor_knot_spacing_m: CORRIDOR_KNOT_SPACING_M,
            corridor_air_speed_low_m_s: CORRIDOR_AIR_SPEED_LOW_M_S,
            corridor_air_speed_high_m_s: CORRIDOR_AIR_SPEED_HIGH_M_S,
            corridor_path_sine_low: CORRIDOR_PATH_SINE_LOW,
            corridor_path_sine_high: CORRIDOR_PATH_SINE_HIGH,
            energy_scale_j_kg: ENERGY_SCALE_J_KG,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if capsule.is_valid() {
            capsule
        } else {
            ::core::panic!("InSight entry capsule failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !same_bits(self.capsule_mass_kg, CAPSULE_MASS_KG)
            || !same_bits(self.reference_area_m2, REFERENCE_AREA_M2)
            || !same_bits(self.entry_radius_m, ENTRY_RADIUS_M)
            || !same_bits(self.entry_radial_speed_m_s, ENTRY_RADIAL_SPEED_M_S)
            || !same_bits(self.entry_transverse_speed_m_s, ENTRY_TRANSVERSE_SPEED_M_S)
            || !same_bits(self.datum_radius_m, DATUM_RADIUS_M)
            || !same_bits(
                self.mars_gravitational_parameter_m3_s2,
                MARS_GRAVITATIONAL_PARAMETER_M3_S2,
            )
            || !same_bits(self.air_rotation_rate_rad_s, AIR_ROTATION_RATE_RAD_S)
            || !same_bits(self.standard_gravity_m_s2, STANDARD_GRAVITY_M_S2)
            || !same_bits(self.altitude_ceiling_m, ALTITUDE_CEILING_M)
            || !same_bits(self.altitude_floor_m, ALTITUDE_FLOOR_M)
            || !same_bits(self.mach_floor, MACH_FLOOR)
            || !same_bits(self.air_speed_ceiling_m_s, AIR_SPEED_CEILING_M_S)
            || !same_bits(self.lift_over_drag_limit, LIFT_OVER_DRAG_LIMIT)
            || !same_bits(self.table_knot_spacing_m, TABLE_KNOT_SPACING_M)
            || !same_bits(self.drag_mach_table_top, DRAG_MACH_TABLE_TOP)
            || !same_bits(
                self.drag_speed_table_mach_floor,
                DRAG_SPEED_TABLE_MACH_FLOOR,
            )
            || !same_bits(self.compared_altitude_floor_m, COMPARED_ALTITUDE_FLOOR_M)
            || !same_bits(self.compared_mach_floor, COMPARED_MACH_FLOOR)
            || !same_bits(self.corridor_bottom_m, CORRIDOR_BOTTOM_M)
            || !same_bits(self.corridor_knot_spacing_m, CORRIDOR_KNOT_SPACING_M)
            || !same_bits(self.energy_scale_j_kg, ENERGY_SCALE_J_KG)
            || !same_bits(
                self.residual_conservation_limit,
                RESIDUAL_CONSERVATION_LIMIT,
            )
        {
            return false;
        }
        let mut index = 0;
        while index < KNOTS {
            if !same_bits(self.air_density_kg_m3[index], AIR_DENSITY_KG_M3[index])
                || !same_bits(self.sound_speed_m_s[index], SOUND_SPEED_M_S[index])
            {
                return false;
            }
            if !(self.air_density_kg_m3[index] > 0.0 && self.sound_speed_m_s[index] > 0.0) {
                return false;
            }
            if index > 0 && !(self.air_density_kg_m3[index] < self.air_density_kg_m3[index - 1]) {
                return false;
            }
            index += 1;
        }
        let mut index = 0;
        while index < DRAG_SPEED_ENTRIES {
            if !same_bits(self.drag_speed_km_s[index], DRAG_SPEED_KM_S[index])
                || !same_bits(self.drag_by_speed[index], DRAG_BY_SPEED[index])
            {
                return false;
            }
            if !(self.drag_by_speed[index] > 0.0) {
                return false;
            }
            if index > 0 && !(self.drag_speed_km_s[index] > self.drag_speed_km_s[index - 1]) {
                return false;
            }
            index += 1;
        }
        let mut index = 0;
        while index < DRAG_MACH_ENTRIES {
            if !same_bits(self.drag_mach[index], DRAG_MACH[index])
                || !same_bits(self.drag_by_mach[index], DRAG_BY_MACH[index])
            {
                return false;
            }
            if !(self.drag_by_mach[index] > 0.0) {
                return false;
            }
            if index > 0 && !(self.drag_mach[index] > self.drag_mach[index - 1]) {
                return false;
            }
            index += 1;
        }
        let mut index = 0;
        while index < CORRIDOR_KNOTS {
            if !same_bits(
                self.corridor_air_speed_low_m_s[index],
                CORRIDOR_AIR_SPEED_LOW_M_S[index],
            ) || !same_bits(
                self.corridor_air_speed_high_m_s[index],
                CORRIDOR_AIR_SPEED_HIGH_M_S[index],
            ) || !same_bits(self.corridor_path_sine_low[index], CORRIDOR_PATH_SINE_LOW[index])
                || !same_bits(self.corridor_path_sine_high[index], CORRIDOR_PATH_SINE_HIGH[index])
            {
                return false;
            }
            if !(self.corridor_air_speed_low_m_s[index] < self.corridor_air_speed_high_m_s[index]
                && self.corridor_path_sine_low[index] < self.corridor_path_sine_high[index])
            {
                return false;
            }
            index += 1;
        }
        let corridor_top_m =
            self.corridor_bottom_m + ((CORRIDOR_KNOTS - 1) as f64) * self.corridor_knot_spacing_m;
        let top_knot_m = self.altitude_floor_m + ((KNOTS - 1) as f64) * self.table_knot_spacing_m;
        let entry_altitude_m = self.entry_radius_m - self.datum_radius_m;
        // The air tables reach past the ceiling, the speed drag table ends at
        // the air-speed ceiling, and the Mach drag table starts at or below
        // the Mach floor and ends where the read by Mach number ends. So no
        // read can fall off a table's end but the one the header states.
        top_knot_m > self.altitude_ceiling_m + self.table_knot_spacing_m
            && same_bits(
                self.drag_speed_km_s[DRAG_SPEED_ENTRIES - 1] * 1000.0,
                self.air_speed_ceiling_m_s,
            )
            && self.drag_mach[0] <= self.mach_floor
            && same_bits(
                self.drag_mach[DRAG_MACH_ENTRIES - 1],
                self.drag_mach_table_top,
            )
            && self.drag_speed_table_mach_floor > self.drag_mach_table_top
            && entry_altitude_m > self.altitude_floor_m
            && entry_altitude_m < self.altitude_ceiling_m
            && self.compared_altitude_floor_m > self.altitude_floor_m
            && self.compared_mach_floor > self.mach_floor
            && self.corridor_bottom_m >= self.altitude_floor_m
            && self.corridor_bottom_m <= self.compared_altitude_floor_m
            && same_bits(corridor_top_m, CORRIDOR_TOP_M)
            && entry_altitude_m <= corridor_top_m
            && corridor_top_m <= self.altitude_ceiling_m
            && self.capsule_mass_kg > 0.0
            && self.reference_area_m2 > 0.0
            && self.lift_over_drag_limit > 0.0
            && self.energy_scale_j_kg > 0.0
            && self.residual_conservation_limit > 0.0
    }

    /// [`GATE_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.is_valid() {
            Some(GATE_DT_S)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`GATE_DT_S`] divided by 2^20,
    /// 9.5e-8 s. At the ceiling step the capsule falls 120 m at the entry
    /// state. A million times smaller it falls 0.11 mm, which the 64-bit
    /// radius, 3522 km, still resolves to five figures. Under that a step is
    /// mostly rounding. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }
}

/// Integrated capsule. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapsuleState {
    /// Time since the state the ledger started from.
    pub t_s: f64,
    /// Distance from the centre of Mars.
    pub radius_m: f64,
    /// Negative going down. Fixed frame.
    pub radial_speed_m_s: f64,
    /// Fixed frame.
    pub transverse_speed_m_s: f64,
    /// Angle travelled about the centre of Mars. Fixed frame.
    pub angle_rad: f64,
    /// Energy per kilogram removed by drag, integrated by the step.
    pub drag_loss_j_kg: f64,
    /// `J` at the state the ledger started from.
    pub energy_anchor_j_kg: f64,
    /// Signed `J − J_anchor + Q`. Positive is created energy.
    pub energy_defect_j_kg: f64,
    pub residual_conservation_error: f64,
    pub altitude_m: f64,
    /// `angle_rad × DATUM_RADIUS_M`.
    pub ground_distance_m: f64,
    pub air_speed_m_s: f64,
    pub mach: f64,
    /// Drag and lift together, in Earth g: what an accelerometer on the capsule reads.
    pub load_g: f64,
    /// `½ ρ u²`. Reported only.
    pub dynamic_pressure_pa: f64,
    pub lift_acting: bool,
}

impl CapsuleState {
    /// The locked entry state: radius 3522.2 km, 126.5 km above the datum.
    #[must_use]
    pub fn try_entry_interface(capsule: &CapsuleInsight) -> Option<Self> {
        Self::try_state(
            capsule,
            capsule.entry_radius_m,
            capsule.entry_radial_speed_m_s,
            capsule.entry_transverse_speed_m_s,
        )
    }

    /// Anchor the ledger at this radius and these two speeds. `None` where
    /// [`CapsuleState::start`] refuses.
    #[must_use]
    pub fn try_state(
        capsule: &CapsuleInsight,
        radius_m: f64,
        radial_speed_m_s: f64,
        transverse_speed_m_s: f64,
    ) -> Option<Self> {
        Self::start(capsule, radius_m, radial_speed_m_s, transverse_speed_m_s).ok()
    }

    /// Anchor the ledger at this radius and these two speeds, or say by name
    /// why not: the altitude must be from 5.7 km to 128 km, the Mach number
    /// at least 1.46, the air speed at most 5850 m/s, and the state inside
    /// the corridor the compared runs flew.
    pub fn start(
        capsule: &CapsuleInsight,
        radius_m: f64,
        radial_speed_m_s: f64,
        transverse_speed_m_s: f64,
    ) -> Result<Self, StepReject> {
        if !capsule.is_valid() {
            return Err(StepReject::InvalidCapsule);
        }
        if !radius_m.is_finite() || !radial_speed_m_s.is_finite() || !transverse_speed_m_s.is_finite()
        {
            return Err(StepReject::NonFiniteState);
        }
        let air = air_at(capsule, radius_m, radial_speed_m_s, transverse_speed_m_s)?;
        in_corridor(capsule, &air)?;
        let energy =
            rotating_frame_energy_j_kg(capsule, radius_m, radial_speed_m_s, transverse_speed_m_s);
        let mut state = Self {
            t_s: 0.0,
            radius_m,
            radial_speed_m_s,
            transverse_speed_m_s,
            angle_rad: 0.0,
            drag_loss_j_kg: 0.0,
            energy_anchor_j_kg: energy,
            energy_defect_j_kg: 0.0,
            residual_conservation_error: 0.0,
            altitude_m: 0.0,
            ground_distance_m: 0.0,
            air_speed_m_s: 0.0,
            mach: 0.0,
            load_g: 0.0,
            dynamic_pressure_pa: 0.0,
            lift_acting: false,
        };
        refresh(capsule, &mut state, &air, 0.0);
        if state_is_finite(&state) {
            Ok(state)
        } else {
            Err(StepReject::NonFiniteState)
        }
    }
}

/// Advance one RK4 step and write the frame.
///
/// `load` is the lift over drag held through the step. A rejected step leaves
/// `state` and `frame` as they were.
#[must_use]
pub fn step(
    capsule: &CapsuleInsight,
    state: &mut CapsuleState,
    frame: &mut LastStateFrame64,
    load: CapsuleLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(capsule, state, load, dt_s)?;
    let predicted = integrate(capsule, recorded(state), load.lift_over_drag, dt_s)?;
    commit(capsule, state, frame, load.lift_over_drag, dt_s, predicted)
}

fn validate(
    capsule: &CapsuleInsight,
    state: &CapsuleState,
    load: CapsuleLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.lift_over_drag.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !capsule.is_valid() {
        return Err(StepReject::InvalidCapsule);
    }
    if !(load.lift_over_drag >= -capsule.lift_over_drag_limit
        && load.lift_over_drag <= capsule.lift_over_drag_limit)
    {
        return Err(StepReject::LiftOutsideCard);
    }
    let air = air_at(
        capsule,
        state.radius_m,
        state.radial_speed_m_s,
        state.transverse_speed_m_s,
    )?;
    in_corridor(capsule, &air)
}

/// One classical RK4 step on `(r, v_r, v_t, θ, Q)`. A stage outside the card
/// is refused; no density and no drag is made up outside the tables.
fn integrate(
    capsule: &CapsuleInsight,
    y: [f64; 5],
    lift_over_drag: f64,
    dt_s: f64,
) -> Result<[f64; 5], StepReject> {
    let k1 = rates(capsule, y, lift_over_drag)?;
    let k2 = rates(capsule, axpy(y, dt_s * 0.5, k1), lift_over_drag)?;
    let k3 = rates(capsule, axpy(y, dt_s * 0.5, k2), lift_over_drag)?;
    let k4 = rates(capsule, axpy(y, dt_s, k3), lift_over_drag)?;
    let mut next = y;
    let mut index = 0;
    while index < 5 {
        let increment = k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index];
        if increment != 0.0 {
            next[index] += (dt_s / 6.0) * increment;
        }
        index += 1;
    }
    if finite5(next) {
        Ok(next)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

fn commit(
    capsule: &CapsuleInsight,
    state: &mut CapsuleState,
    frame: &mut LastStateFrame64,
    lift_over_drag: f64,
    dt_s: f64,
    predicted: [f64; 5],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite5(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    let air = air_at(capsule, predicted[0], predicted[1], predicted[2])?;
    let scale = capsule.energy_scale_j_kg;
    if !(scale > 0.0 && scale.is_finite()) {
        return Err(StepReject::InvalidCapsule);
    }
    let energy = rotating_frame_energy_j_kg(capsule, predicted[0], predicted[1], predicted[2]);
    let defect = energy - state.energy_anchor_j_kg + predicted[4];
    if !defect.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let gate = capsule.residual_conservation_limit * scale;
    if defect > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if defect < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    in_corridor(capsule, &air)?;
    state.radius_m = predicted[0];
    state.radial_speed_m_s = predicted[1];
    state.transverse_speed_m_s = predicted[2];
    state.angle_rad = predicted[3];
    state.drag_loss_j_kg = predicted[4];
    state.energy_defect_j_kg = defect;
    state.residual_conservation_error = abs_f64(defect) / scale;
    state.t_s += dt_s;
    refresh(capsule, state, &air, lift_over_drag);
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    publish(state, frame);
    Ok(())
}

/// Write the fields that follow from the phase and from the lift of the step.
fn refresh(capsule: &CapsuleInsight, state: &mut CapsuleState, air: &Air, lift_over_drag: f64) {
    let u = air.air_speed_m_s;
    state.altitude_m = air.altitude_m;
    state.ground_distance_m = state.angle_rad * capsule.datum_radius_m;
    state.air_speed_m_s = u;
    state.mach = air.mach;
    state.load_g = air.drag_acceleration_m_s2 * (1.0 + lift_over_drag * lift_over_drag).sqrt()
        / capsule.standard_gravity_m_s2;
    state.dynamic_pressure_pa = 0.5 * air.density_kg_m3 * u * u;
    state.lift_acting = lift_over_drag != 0.0;
}

fn publish(state: &CapsuleState, frame: &mut LastStateFrame64) {
    let mut flags = 0;
    if state.lift_acting {
        flags |= FLAG_LIFT_ACTING;
    }
    frame.t = state.t_s;
    frame.pos[0] = state.altitude_m as f32;
    frame.pos[1] = state.ground_distance_m as f32;
    frame.pos[2] = state.mach as f32;
    frame.vel[0] = state.air_speed_m_s as f32;
    frame.vel[1] = state.radial_speed_m_s as f32;
    frame.vel[2] = state.dynamic_pressure_pa as f32;
    frame.force_torque = state.load_g as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = flags;
}

/// The air at one phase.
struct Air {
    altitude_m: f64,
    radial_m_s: f64,
    transverse_m_s: f64,
    air_speed_m_s: f64,
    density_kg_m3: f64,
    mach: f64,
    drag_acceleration_m_s2: f64,
}

/// The air the capsule meets at this radius and these two speeds. An
/// altitude, a Mach number or an air speed outside the card is refused by
/// name: outside the tables first, then past the compared floor.
fn air_at(
    capsule: &CapsuleInsight,
    radius_m: f64,
    radial_speed_m_s: f64,
    transverse_speed_m_s: f64,
) -> Result<Air, StepReject> {
    let altitude_m = radius_m - capsule.datum_radius_m;
    if !altitude_m.is_finite() || !radial_speed_m_s.is_finite() || !transverse_speed_m_s.is_finite()
    {
        return Err(StepReject::NonFiniteState);
    }
    if !(altitude_m >= capsule.altitude_floor_m && altitude_m <= capsule.altitude_ceiling_m) {
        return Err(StepReject::AltitudeOutsideCard);
    }
    let radial_m_s = radial_speed_m_s;
    let transverse_m_s = transverse_speed_m_s - capsule.air_rotation_rate_rad_s * radius_m;
    let air_speed_m_s = (radial_m_s * radial_m_s + transverse_m_s * transverse_m_s).sqrt();
    let Some(density_kg_m3) = between_knots(capsule, &capsule.air_density_kg_m3, altitude_m) else {
        return Err(StepReject::AltitudeOutsideCard);
    };
    let Some(sound_speed_m_s) = between_knots(capsule, &capsule.sound_speed_m_s, altitude_m) else {
        return Err(StepReject::AltitudeOutsideCard);
    };
    let mach = air_speed_m_s / sound_speed_m_s;
    if !(air_speed_m_s.is_finite() && mach.is_finite()) {
        return Err(StepReject::NonFiniteState);
    }
    if !(mach >= capsule.mach_floor) {
        return Err(StepReject::MachOutsideCard);
    }
    if !(air_speed_m_s <= capsule.air_speed_ceiling_m_s) {
        return Err(StepReject::AirSpeedOutsideCard);
    }
    if !(altitude_m >= capsule.compared_altitude_floor_m && mach >= capsule.compared_mach_floor) {
        return Err(StepReject::PastComparedFlight);
    }
    let drag_coefficient = drag_at(capsule, air_speed_m_s, mach)?;
    let drag_acceleration_m_s2 = 0.5
        * density_kg_m3
        * air_speed_m_s
        * air_speed_m_s
        * drag_coefficient
        * capsule.reference_area_m2
        / capsule.capsule_mass_kg;
    if !drag_acceleration_m_s2.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    Ok(Air {
        altitude_m,
        radial_m_s,
        transverse_m_s,
        air_speed_m_s,
        density_kg_m3,
        mach,
        drag_acceleration_m_s2,
    })
}

/// The corridor the compared runs flew, read at this air's altitude by
/// straight lines between knots. A state on a bound, or exactly on the top
/// knot, is inside.
fn in_corridor(capsule: &CapsuleInsight, air: &Air) -> Result<(), StepReject> {
    let knots_up = (air.altitude_m - capsule.corridor_bottom_m) / capsule.corridor_knot_spacing_m;
    if !(knots_up >= 0.0 && knots_up <= (CORRIDOR_KNOTS - 1) as f64) {
        return Err(StepReject::OffComparedCorridor);
    }
    let mut index = knots_up as usize;
    if index == CORRIDOR_KNOTS - 1 {
        index -= 1;
    }
    let share = knots_up - (index as f64);
    let read = |table: &[f64; CORRIDOR_KNOTS]| table[index] + (table[index + 1] - table[index]) * share;
    let sine = air.radial_m_s / air.air_speed_m_s;
    if !(air.air_speed_m_s >= read(&capsule.corridor_air_speed_low_m_s)
        && air.air_speed_m_s <= read(&capsule.corridor_air_speed_high_m_s)
        && sine >= read(&capsule.corridor_path_sine_low)
        && sine <= read(&capsule.corridor_path_sine_high))
    {
        return Err(StepReject::OffComparedCorridor);
    }
    Ok(())
}

/// Straight line between the two knots either side of `altitude_m`. `None`
/// when the altitude is not inside the table: nothing is held at its ends.
fn between_knots(capsule: &CapsuleInsight, table: &[f64; KNOTS], altitude_m: f64) -> Option<f64> {
    let knots_up = (altitude_m - capsule.altitude_floor_m) / capsule.table_knot_spacing_m;
    if !(knots_up >= 0.0) {
        return None;
    }
    let index = knots_up as usize;
    if index + 1 >= KNOTS {
        return None;
    }
    Some(table[index] + (table[index + 1] - table[index]) * (knots_up - (index as f64)))
}

/// Straight line between two entries of a drag table. `None` outside the
/// table: nothing is held at its ends.
fn between_entries(arguments: &[f64], values: &[f64], x: f64) -> Option<f64> {
    let last = arguments.len() - 1;
    if !(x >= arguments[0] && x <= arguments[last]) {
        return None;
    }
    if x == arguments[last] {
        return Some(values[last]);
    }
    let mut index = 0;
    while x >= arguments[index + 1] {
        index += 1;
    }
    Some(
        values[index]
            + (values[index + 1] - values[index]) / (arguments[index + 1] - arguments[index])
                * (x - arguments[index]),
    )
}

/// Axial-force coefficient at zero angle of attack. At Mach 6.3 and below it
/// is read by Mach number. Above, it is read by air speed, with the speed
/// table's first entry held below 1.8 km/s: the one place the card reads past
/// a table, stated in the module header. From Mach 6.3 to Mach 8.8 the value
/// is a straight line in Mach number from the Mach table's top entry to the
/// value read by speed.
fn drag_at(capsule: &CapsuleInsight, air_speed_m_s: f64, mach: f64) -> Result<f64, StepReject> {
    if mach <= capsule.drag_mach_table_top {
        let Some(value) = between_entries(&capsule.drag_mach, &capsule.drag_by_mach, mach) else {
            return Err(StepReject::MachOutsideCard);
        };
        return Ok(value);
    }
    let km_s = air_speed_m_s / 1000.0;
    let by_speed = if km_s <= capsule.drag_speed_km_s[0] {
        capsule.drag_by_speed[0]
    } else {
        let Some(value) = between_entries(&capsule.drag_speed_km_s, &capsule.drag_by_speed, km_s)
        else {
            return Err(StepReject::AirSpeedOutsideCard);
        };
        value
    };
    if mach >= capsule.drag_speed_table_mach_floor {
        return Ok(by_speed);
    }
    let top = capsule.drag_by_mach[DRAG_MACH_ENTRIES - 1];
    Ok(top
        + (by_speed - top)
            * ((mach - capsule.drag_mach_table_top)
                / (capsule.drag_speed_table_mach_floor - capsule.drag_mach_table_top)))
}

/// Rates of `(r, v_r, v_t, θ, Q)`.
fn rates(
    capsule: &CapsuleInsight,
    y: [f64; 5],
    lift_over_drag: f64,
) -> Result<[f64; 5], StepReject> {
    let radius_m = y[0];
    let radial_speed_m_s = y[1];
    let transverse_speed_m_s = y[2];
    let air = air_at(capsule, radius_m, radial_speed_m_s, transverse_speed_m_s)?;
    let drag = air.drag_acceleration_m_s2;
    let drag_radial = -drag * air.radial_m_s / air.air_speed_m_s;
    let drag_transverse = -drag * air.transverse_m_s / air.air_speed_m_s;
    // Lift is drag turned a quarter turn in the flight plane, away from Mars
    // for a positive load.
    let lift_radial = lift_over_drag * drag * air.transverse_m_s / air.air_speed_m_s;
    let lift_transverse = -(lift_over_drag * drag * air.radial_m_s / air.air_speed_m_s);
    let out = [
        radial_speed_m_s,
        transverse_speed_m_s * transverse_speed_m_s / radius_m
            - capsule.mars_gravitational_parameter_m3_s2 / (radius_m * radius_m)
            + drag_radial
            + lift_radial,
        -radial_speed_m_s * transverse_speed_m_s / radius_m + drag_transverse + lift_transverse,
        transverse_speed_m_s / radius_m,
        drag * air.air_speed_m_s,
    ];
    if finite5(out) {
        Ok(out)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

/// `J = ½ (v_r² + v_t²) − GM/r − Ω r v_t`.
fn rotating_frame_energy_j_kg(
    capsule: &CapsuleInsight,
    radius_m: f64,
    radial_speed_m_s: f64,
    transverse_speed_m_s: f64,
) -> f64 {
    0.5 * (radial_speed_m_s * radial_speed_m_s + transverse_speed_m_s * transverse_speed_m_s)
        - capsule.mars_gravitational_parameter_m3_s2 / radius_m
        - capsule.air_rotation_rate_rad_s * radius_m * transverse_speed_m_s
}

fn axpy(y: [f64; 5], scale: f64, k: [f64; 5]) -> [f64; 5] {
    [
        mul_dt(y[0], scale, k[0]),
        mul_dt(y[1], scale, k[1]),
        mul_dt(y[2], scale, k[2]),
        mul_dt(y[3], scale, k[3]),
        mul_dt(y[4], scale, k[4]),
    ]
}

fn mul_dt(y: f64, scale: f64, k: f64) -> f64 {
    if k == 0.0 { y } else { y + scale * k }
}

fn recorded(state: &CapsuleState) -> [f64; 5] {
    [
        state.radius_m,
        state.radial_speed_m_s,
        state.transverse_speed_m_s,
        state.angle_rad,
        state.drag_loss_j_kg,
    ]
}

fn finite5(x: [f64; 5]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite() && x[3].is_finite() && x[4].is_finite()
}

fn same_bits(recorded: f64, locked: f64) -> bool {
    recorded.to_bits() == locked.to_bits()
}

fn abs_f64(value: f64) -> f64 {
    if value < 0.0 { -value } else { value }
}

fn state_is_finite(state: &CapsuleState) -> bool {
    state.t_s.is_finite()
        && state.radius_m.is_finite()
        && state.radial_speed_m_s.is_finite()
        && state.transverse_speed_m_s.is_finite()
        && state.angle_rad.is_finite()
        && state.drag_loss_j_kg.is_finite()
        && state.energy_anchor_j_kg.is_finite()
        && state.energy_defect_j_kg.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.altitude_m.is_finite()
        && state.ground_distance_m.is_finite()
        && state.air_speed_m_s.is_finite()
        && state.mach.is_finite()
        && state.load_g.is_finite()
        && state.dynamic_pressure_pa.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        AIR_DENSITY_KG_M3, AIR_SPEED_CEILING_M_S, ALTITUDE_CEILING_M, ALTITUDE_FLOOR_M,
        CAPSULE_MASS_KG, CapsuleInsight, CapsuleLoad, CapsuleState, DATUM_RADIUS_M, DRAG_BY_MACH,
        DRAG_BY_SPEED, DRAG_MACH, DRAG_SPEED_KM_S, ENERGY_SCALE_J_KG, ENTRY_RADIUS_M,
        COMPARED_ALTITUDE_FLOOR_M, COMPARED_MACH_FLOOR, CORRIDOR_AIR_SPEED_HIGH_M_S,
        CORRIDOR_AIR_SPEED_LOW_M_S, CORRIDOR_BOTTOM_M, CORRIDOR_KNOT_SPACING_M, CORRIDOR_KNOTS,
        CORRIDOR_PATH_SINE_HIGH, CORRIDOR_PATH_SINE_LOW, CORRIDOR_TOP_M, FLAG_LIFT_ACTING,
        GATE_DT_S, KNOTS, LIFT_OVER_DRAG_LIMIT, MACH_FLOOR,
        REFERENCE_AREA_M2, RESIDUAL_CONSERVATION_LIMIT, SOUND_SPEED_M_S, STANDARD_GRAVITY_M_S2,
        StepReject, abs_f64, axpy, between_entries, between_knots, commit, drag_at, integrate,
        rates, recorded, rotating_frame_energy_j_kg, step, validate,
    };
    use crate::frame::LastStateFrame64;

    const DT: f64 = GATE_DT_S;
    const NO_LIFT: CapsuleLoad = CapsuleLoad {
        lift_over_drag: 0.0,
    };
    /// The vertical lift over drag measured on the first 150 s of the flight.
    const FLIGHT_LIFT: CapsuleLoad = CapsuleLoad {
        lift_over_drag: -0.031,
    };

    /// The model in table form, RK4 at 0.1 s from the locked entry state with
    /// no lift (`mars_entry_insight/analysis/out/card_reference.json`, run
    /// R0). Each row is the step number, then `t_s`, `radius_m`,
    /// `radial_speed_m_s`, `transverse_speed_m_s`, `angle_rad`,
    /// `drag_loss_j_kg`, `altitude_m`, `ground_distance_m`, `air_speed_m_s`,
    /// `mach`, `load_g`, `dynamic_pressure_pa`, `energy_defect_j_kg`, then
    /// the flags.
    const REFERENCE_NO_LIFT: [(u32, [f64; 13], u64); 10] = [
        (
            0,
            [
                0.0,
                3522199.9999655997,
                -1202.1585438056352,
                5633.705477158609,
                0.0,
                0.0,
                126485.99996559974,
                0.0,
                5523.356032667084,
                28.163443041406488,
                0.00035044578065886905,
                0.21879703164537836,
                0.0,
            ],
            0,
        ),
        (
            1,
            [
                0.1,
                3522079.8119101003,
                -1201.6025545835907,
                5633.8973848371625,
                0.0001599539370008196,
                1.9118762335366943,
                126365.81191010028,
                543.1578232288011,
                5523.430444930971,
                28.163822467527705,
                0.0003555094019943801,
                0.2219584488423641,
                -2.0862183092162923e-09,
            ],
            0,
        ),
        (
            300,
            [
                30.000000000000156,
                3488668.71729925,
                -1032.1509543308118,
                5686.391462663106,
                0.048454582883613075,
                8238.263361968875,
                92954.71729924995,
                164537.90546204528,
                5542.8881662935755,
                28.26303686255678,
                0.020081414971906558,
                12.537614174813486,
                2.6851921575143933e-08,
            ],
            0,
        ),
        (
            600,
            [
                60.00000000000058,
                3460381.4713646816,
                -849.5329567148381,
                5678.843879007764,
                0.09764329172159054,
                310410.4102399478,
                64667.47136468161,
                331568.6927050891,
                5506.272711444319,
                27.608181698636432,
                0.6111186400367014,
                381.5453111514208,
                7.526250556111336e-08,
            ],
            0,
        ),
        (
            900,
            [
                89.99999999999916,
                3438351.540226083,
                -599.1733465100493,
                4980.012098578582,
                0.1450723775723969,
                4121198.188153105,
                42637.54022608278,
                492624.3035358742,
                4780.725810827086,
                23.279898481267253,
                5.559572161324513,
                3467.311928492971,
                5.988404154777527e-06,
            ],
            0,
        ),
        (
            1047,
            [
                104.69999999999833,
                3430684.2549709403,
                -445.56781360090486,
                4008.244047527673,
                0.16441625418841735,
                8364115.187637469,
                34970.254970940296,
                558310.5761751675,
                3798.0167371588745,
                17.843665376363507,
                7.600699812494663,
                4764.42848022584,
                1.0902062058448792e-05,
            ],
            0,
        ),
        (
            1200,
            [
                119.99999999999746,
                3424885.3751694215,
                -321.27873317082407,
                2940.7200309036903,
                0.1798620377314664,
                11888404.99954003,
                29171.375169421546,
                610760.0395932687,
                2723.6807508459856,
                12.42469623471724,
                6.49085413897793,
                4213.083097430668,
                1.0760501027107239e-05,
            ],
            0,
        ),
        (
            1500,
            [
                149.99999999999577,
                3417272.0559563925,
                -209.39490402188926,
                1577.5039684750898,
                0.19893330789962146,
                14702988.589511152,
                21558.05595639255,
                675520.6187010552,
                1358.2126382426522,
                6.013982758415779,
                3.1105201110448815,
                2086.471568166837,
                1.329369843006134e-05,
            ],
            0,
        ),
        (
            1800,
            [
                179.99999999999406,
                3411480.38296335,
                -184.1878375782129,
                951.0517968920226,
                0.20969648780248526,
                15373308.060041398,
                15766.382963350043,
                712069.2993817284,
                739.2351811967462,
                3.203215948322494,
                1.493610870562361,
                1018.398876072949,
                1.4074146747589111e-05,
            ],
            0,
        ),
        (
            2100,
            [
                209.99999999999235,
                3405982.411608315,
                -184.07624925810424,
                639.0476108091528,
                0.2165501953549296,
                15568048.548827544,
                10268.411608315073,
                735342.5300694694,
                444.2290717422121,
                1.8901691460409544,
                0.8470432907220952,
                582.955023453972,
                1.4217570424079895e-05,
            ],
            0,
        ),
    ];

    /// The same with a lift over drag of −0.031 held through the run (run R1).
    const REFERENCE_FLIGHT_LIFT: [(u32, [f64; 13], u64); 10] = [
        (
            0,
            [
                0.0,
                3522199.9999655997,
                -1202.1585438056352,
                5633.705477158609,
                0.0,
                0.0,
                126485.99996559974,
                0.0,
                5523.356032667084,
                28.163443041406488,
                0.00035044578065886905,
                0.21879703164537836,
                0.0,
            ],
            0,
        ),
        (
            1,
            [
                0.1,
                3522079.811909578,
                -1201.6025650572358,
                5633.897382503046,
                0.0001599539369677653,
                1.9118762335766315,
                126365.81190957781,
                543.1578231165581,
                5523.430444931297,
                28.163822467529364,
                0.0003556801832640988,
                0.221958448856344,
                1.2133478666243036e-09,
            ],
            1,
        ),
        (
            300,
            [
                30.000000000000156,
                3488668.4010492493,
                -1032.196375420597,
                5686.383007931614,
                0.04845456632108651,
                8238.41375240364,
                92954.40104924934,
                164537.84922044195,
                5542.888338987707,
                28.26303774311949,
                0.020091841335925592,
                12.538100651726557,
                -4.127286956645548e-09,
            ],
            1,
        ),
        (
            600,
            [
                60.00000000000058,
                3460366.6324246046,
                -851.2434113290243,
                5678.537708471415,
                0.0976426010242079,
                310668.7523612937,
                64652.63242460461,
                331566.3472943171,
                5506.235388119282,
                27.6066858966826,
                0.6124273263052193,
                382.17878245696795,
                3.474997356534004e-08,
            ],
            1,
        ),
        (
            900,
            [
                89.99999999999916,
                3438074.0855084658,
                -620.974702784986,
                4966.5571471685835,
                0.14504300247116542,
                4172534.6916054133,
                42360.08550846577,
                492524.554093371,
                4770.185281200857,
                23.21119092230692,
                5.707731143991437,
                3557.8197150581805,
                5.9506855905056e-06,
            ],
            1,
        ),
        (
            1065,
            [
                106.49999999999822,
                3429076.7754394948,
                -471.62913374209654,
                3817.5466212247798,
                0.16629985236524006,
                9058680.605292186,
                33362.775439494755,
                564706.7368745788,
                3612.1257195146964,
                16.798938744259473,
                7.951853813798471,
                5004.898555258766,
                1.1244788765907288e-05,
            ],
            1,
        ),
        (
            1200,
            [
                119.99999999999746,
                3423410.1922500995,
                -372.8571168730602,
                2829.8133042678364,
                0.179347674481865,
                12169420.060985984,
                27696.1922500995,
                609013.4091055117,
                2620.522035492663,
                11.883409210850454,
                6.871674858956486,
                4469.936882987826,
                1.1270865797996521e-05,
            ],
            1,
        ),
        (
            1500,
            [
                149.99999999999577,
                3414350.310580272,
                -250.48380550830655,
                1411.4164491298152,
                0.19712906977437616,
                14913073.713905334,
                18636.310580272228,
                669393.942039826,
                1202.466537020811,
                5.263922585952384,
                3.128952814442175,
                2107.510557312944,
                1.4899298548698425e-05,
            ],
            1,
        ),
        (
            1800,
            [
                179.99999999999406,
                3407600.089655964,
                -207.34547483502857,
                806.4878752325483,
                0.20651177802895623,
                15475901.472836006,
                11886.08965596417,
                701254.9358178191,
                608.0684419373678,
                2.601898120519608,
                1.3989069644161922,
                955.9461136839665,
                1.5852972865104675e-05,
            ],
            1,
        ),
        (
            2100,
            [
                209.99999999999235,
                3401623.5704418966,
                -193.90058439141464,
                532.0380862597582,
                0.21226258710228102,
                15619682.81760923,
                5909.5704418965615,
                720783.0386994351,
                355.1837672284149,
                1.4839447171651743,
                0.7505105199533,
                526.080561223331,
                1.598149538040161e-05,
            ],
            1,
        ),
    ];

    /// The last landed step before `PastComparedFlight`: with no lift at
    /// 227.3 s and Mach 1.46, with the flight's lift at 211.0 s and Mach 1.46.
    const LAST_STEP_NO_LIFT: u32 = 2273;
    const LAST_STEP_FLIGHT_LIFT: u32 = 2110;
    const PEAK_STEP_NO_LIFT: u32 = 1047;
    const PEAK_STEP_FLIGHT_LIFT: u32 = 1065;
    /// The last row of the flight table, parachute deployment: its time, its
    /// speed through the air, and its height above the datum.
    const FLIGHT_DEPLOYMENT_T_S: f64 = 209.2703;
    const FLIGHT_DEPLOYMENT_AIR_SPEED_M_S: f64 = 352.1169;
    const FLIGHT_DEPLOYMENT_ALTITUDE_M: f64 = 6_164.62;

    /// A radius and two speeds with this altitude, this speed through the
    /// air and this sine of the path angle through the air.
    fn phase_at(capsule: &CapsuleInsight, altitude_m: f64, air_speed_m_s: f64, sine: f64) -> (f64, f64, f64) {
        let radius_m = capsule.datum_radius_m + altitude_m;
        let radial = sine * air_speed_m_s;
        let across = (air_speed_m_s * air_speed_m_s - radial * radial).sqrt();
        (radius_m, radial, across + capsule.air_rotation_rate_rad_s * radius_m)
    }

    /// The corridor's four bounds at an altitude, read as the card reads them.
    fn corridor_at(altitude_m: f64) -> [f64; 4] {
        let knots_up = (altitude_m - CORRIDOR_BOTTOM_M) / CORRIDOR_KNOT_SPACING_M;
        let mut index = knots_up as usize;
        if index == CORRIDOR_KNOTS - 1 {
            index -= 1;
        }
        let share = knots_up - (index as f64);
        let read = |table: &[f64; CORRIDOR_KNOTS]| table[index] + (table[index + 1] - table[index]) * share;
        [
            read(&CORRIDOR_AIR_SPEED_LOW_M_S),
            read(&CORRIDOR_AIR_SPEED_HIGH_M_S),
            read(&CORRIDOR_PATH_SINE_LOW),
            read(&CORRIDOR_PATH_SINE_HIGH),
        ]
    }

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

    fn state_bits(state: &CapsuleState) -> [u64; 16] {
        [
            state.t_s.to_bits(),
            state.radius_m.to_bits(),
            state.radial_speed_m_s.to_bits(),
            state.transverse_speed_m_s.to_bits(),
            state.angle_rad.to_bits(),
            state.drag_loss_j_kg.to_bits(),
            state.energy_anchor_j_kg.to_bits(),
            state.energy_defect_j_kg.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.altitude_m.to_bits(),
            state.ground_distance_m.to_bits(),
            state.air_speed_m_s.to_bits(),
            state.mach.to_bits(),
            state.load_g.to_bits(),
            state.dynamic_pressure_pa.to_bits(),
            u64::from(state.lift_acting),
        ]
    }

    fn flags_of(state: &CapsuleState) -> u64 {
        let mut flags = 0;
        if state.lift_acting {
            flags |= FLAG_LIFT_ACTING;
        }
        flags
    }

    /// `½ (v_r² + v_t²) − GM/r`: the energy without the term for the turning air.
    fn fixed_frame_energy_j_kg(capsule: &CapsuleInsight, y: [f64; 5]) -> f64 {
        0.5 * (y[1] * y[1] + y[2] * y[2]) - capsule.mars_gravitational_parameter_m3_s2 / y[0]
    }

    /// Explicit Euler on `(r, v_r, v_t, θ, Q)`.
    fn step_euler(
        capsule: &CapsuleInsight,
        state: &mut CapsuleState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(capsule, state, NO_LIFT, dt_s)?;
        let y = recorded(state);
        let start = rates(capsule, y, 0.0)?;
        commit(capsule, state, frame, 0.0, dt_s, axpy(y, dt_s, start))
    }

    /// Honest RK4, then `extra_j_kg` added to the booked drag loss.
    fn step_bias_drag_loss(
        capsule: &CapsuleInsight,
        state: &mut CapsuleState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        extra_j_kg: f64,
    ) -> Result<(), StepReject> {
        validate(capsule, state, NO_LIFT, dt_s)?;
        let mut predicted = integrate(capsule, recorded(state), 0.0, dt_s)?;
        predicted[4] += extra_j_kg;
        commit(capsule, state, frame, 0.0, dt_s, predicted)
    }

    /// The real `validate` against the real capsule, then the RK4 step of
    /// `mover` and the ledger of `keeper`. Either may be a changed copy.
    fn step_with_copies(
        capsule: &CapsuleInsight,
        mover: &CapsuleInsight,
        keeper: &CapsuleInsight,
        state: &mut CapsuleState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(capsule, state, NO_LIFT, dt_s)?;
        let predicted = integrate(mover, recorded(state), 0.0, dt_s)?;
        commit(keeper, state, frame, 0.0, dt_s, predicted)
    }

    /// Rates with the lift laid along the path instead of across it: a force
    /// that does work and that the ledger was not told about.
    fn rates_with_lift_along_the_path(
        capsule: &CapsuleInsight,
        y: [f64; 5],
        lift_over_drag: f64,
    ) -> [f64; 5] {
        let honest = rates(capsule, y, 0.0).unwrap();
        let transverse = y[2] - capsule.air_rotation_rate_rad_s * y[0];
        let speed = (y[1] * y[1] + transverse * transverse).sqrt();
        let drag = honest[4] / speed;
        [
            honest[0],
            honest[1] + lift_over_drag * drag * y[1] / speed,
            honest[2] + lift_over_drag * drag * transverse / speed,
            honest[3],
            honest[4],
        ]
    }

    fn run_to(
        capsule: &CapsuleInsight,
        load: CapsuleLoad,
        steps: u32,
    ) -> (CapsuleState, LastStateFrame64) {
        let mut state = CapsuleState::try_entry_interface(capsule).unwrap();
        let mut frame = blank_frame();
        let mut n = 0;
        while n < steps {
            step(capsule, &mut state, &mut frame, load, DT).unwrap();
            n += 1;
        }
        (state, frame)
    }

    fn check_reference(load: CapsuleLoad, rows: &[(u32, [f64; 13], u64)]) {
        let capsule = CapsuleInsight::entry_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut n = 0u32;
        let mut row = 0;
        while row < rows.len() {
            let (at, want, flags) = rows[row];
            while n < at {
                step(&capsule, &mut state, &mut frame, load, DT).unwrap();
                n += 1;
            }
            let got = [
                state.t_s,
                state.radius_m,
                state.radial_speed_m_s,
                state.transverse_speed_m_s,
                state.angle_rad,
                state.drag_loss_j_kg,
                state.altitude_m,
                state.ground_distance_m,
                state.air_speed_m_s,
                state.mach,
                state.load_g,
                state.dynamic_pressure_pa,
                state.energy_defect_j_kg,
            ];
            let mut place = 0;
            while place < 13 {
                assert_eq!(
                    got[place].to_bits(),
                    want[place].to_bits(),
                    "step {at}, place {place}: {} against {}",
                    got[place],
                    want[place]
                );
                place += 1;
            }
            assert_eq!(flags_of(&state), flags, "flags at step {at}");
            if at > 0 {
                assert_eq!(frame.flags, flags, "frame flags at step {at}");
            }
            row += 1;
        }
        proof_intact(&frame);
    }

    #[test]
    fn constants_and_tables_are_locked() {
        assert_copy::<CapsuleInsight>();
        assert_copy::<CapsuleState>();
        assert_copy::<CapsuleLoad>();
        let capsule = CapsuleInsight::entry_capsule();
        assert!(capsule.is_valid());
        assert_eq!(capsule.gate_dt_s(), Some(0.1));
        assert_eq!(capsule.floor_dt_s(), Some(0.1 / 1_048_576.0));
        let entry = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let held = frame.to_bytes();

        let mut changed = [capsule; 20];
        changed[0].capsule_mass_kg = 608.0;
        changed[1].reference_area_m2 = 5.52;
        changed[2].air_density_kg_m3[137] *= 0.9;
        changed[3].sound_speed_m_s[5] += 1.0;
        changed[4].drag_by_speed[12] = 1.73;
        changed[5].drag_by_mach[3] = 1.55;
        changed[6].datum_radius_m = 3_396_190.0;
        changed[7].air_rotation_rate_rad_s = 0.0;
        changed[8].mars_gravitational_parameter_m3_s2 = 4.282837e13;
        changed[9].lift_over_drag_limit = 0.3;
        changed[10].altitude_floor_m = 0.0;
        changed[11].mach_floor = 0.5;
        changed[12].residual_conservation_limit = 1.0e-3;
        changed[13].entry_radial_speed_m_s = -1000.0;
        changed[14].compared_altitude_floor_m = 4_000.0;
        changed[15].compared_mach_floor = 1.0;
        changed[16].corridor_air_speed_high_m_s[100] += 100.0;
        changed[17].corridor_path_sine_low[0] = -1.0;
        changed[18].corridor_bottom_m = 4_000.0;
        changed[19].corridor_knot_spacing_m = 600.0;
        let mut index = 0;
        while index < changed.len() {
            assert!(!changed[index].is_valid(), "copy {index} passed is_valid");
            assert_eq!(changed[index].gate_dt_s(), None);
            assert_eq!(changed[index].floor_dt_s(), None);
            let mut state = entry;
            let saved = state_bits(&state);
            assert_eq!(
                step(&changed[index], &mut state, &mut frame, NO_LIFT, DT),
                Err(StepReject::InvalidCapsule),
                "copy {index}"
            );
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            assert!(
                CapsuleState::try_state(&changed[index], ENTRY_RADIUS_M, -1000.0, 5000.0).is_none()
            );
            index += 1;
        }
        proof_intact(&frame);
    }

    #[test]
    fn the_air_tables_were_copied_correctly() {
        assert_eq!(KNOTS, 631);
        // First, last and three inner knots of tables/air_for_the_card.csv.
        assert_eq!(AIR_DENSITY_KG_M3[0].to_bits(), 0.009641975984_f64.to_bits());
        assert_eq!(
            AIR_DENSITY_KG_M3[80].to_bits(),
            0.002591094494_f64.to_bits()
        );
        assert_eq!(
            AIR_DENSITY_KG_M3[137].to_bits(),
            0.0009228475262_f64.to_bits()
        );
        assert_eq!(
            AIR_DENSITY_KG_M3[402].to_bits(),
            2.318941244e-06_f64.to_bits()
        );
        assert_eq!(
            AIR_DENSITY_KG_M3[630].to_bits(),
            9.432689721e-09_f64.to_bits()
        );
        assert_eq!(SOUND_SPEED_M_S[0].to_bits(), 241.4749646_f64.to_bits());
        assert_eq!(SOUND_SPEED_M_S[137].to_bits(), 217.0777948_f64.to_bits());
        assert_eq!(SOUND_SPEED_M_S[630].to_bits(), 196.1179258_f64.to_bits());
        // The sums of the two columns, added in knot order.
        let mut density_sum = 0.0;
        let mut sound_sum = 0.0;
        let mut index = 0;
        while index < KNOTS {
            density_sum += AIR_DENSITY_KG_M3[index];
            sound_sum += SOUND_SPEED_M_S[index];
            index += 1;
        }
        assert_eq!(density_sum.to_bits(), 0.5789248776994655_f64.to_bits());
        assert_eq!(sound_sum.to_bits(), 129949.58052480109_f64.to_bits());
        // Above 84.5 km the temperature is held, so the speed of sound is one value.
        let mut index = 403;
        while index < KNOTS {
            assert_eq!(
                SOUND_SPEED_M_S[index].to_bits(),
                SOUND_SPEED_M_S[403].to_bits()
            );
            index += 1;
        }
        // The hydrostatic law holds between neighbouring knots: the density
        // falls by between 1.3 and 3.5 percent in 200 m everywhere.
        let mut index = 1;
        while index < KNOTS {
            let fall = 1.0 - AIR_DENSITY_KG_M3[index] / AIR_DENSITY_KG_M3[index - 1];
            assert!(fall > 0.013 && fall < 0.035, "knot {index}: {fall}");
            index += 1;
        }
        let capsule = CapsuleInsight::entry_capsule();
        // A knot is returned exactly; halfway is the mean; the ends hold nothing.
        assert_eq!(
            between_knots(&capsule, &capsule.air_density_kg_m3, 31_400.0)
                .unwrap()
                .to_bits(),
            AIR_DENSITY_KG_M3[137].to_bits()
        );
        let half = between_knots(&capsule, &capsule.air_density_kg_m3, 31_500.0).unwrap();
        assert_eq!(
            half.to_bits(),
            (AIR_DENSITY_KG_M3[137] + (AIR_DENSITY_KG_M3[138] - AIR_DENSITY_KG_M3[137]) * 0.5)
                .to_bits()
        );
        assert!(between_knots(&capsule, &capsule.air_density_kg_m3, 3_999.9).is_none());
        assert!(between_knots(&capsule, &capsule.air_density_kg_m3, 130_000.0).is_none());
        assert!(between_knots(&capsule, &capsule.air_density_kg_m3, f64::NAN).is_none());
    }

    #[test]
    fn the_drag_tables_return_their_entries_exactly_and_state_the_one_held_value() {
        let capsule = CapsuleInsight::entry_capsule();
        let mut index = 0;
        while index < DRAG_MACH.len() {
            if DRAG_MACH[index] >= MACH_FLOOR {
                let got = drag_at(&capsule, 300.0, DRAG_MACH[index]).unwrap();
                assert_eq!(got.to_bits(), DRAG_BY_MACH[index].to_bits());
            }
            index += 1;
        }
        let mut index = 1;
        while index < DRAG_SPEED_KM_S.len() {
            let got = drag_at(&capsule, DRAG_SPEED_KM_S[index] * 1000.0, 20.0).unwrap();
            assert!(
                abs_f64(got - DRAG_BY_SPEED[index]) < 1.0e-12,
                "entry {index}"
            );
            index += 1;
        }
        // The read by speed holds its first entry below 1.8 km/s.
        assert_eq!(
            drag_at(&capsule, 1500.0, 9.0).unwrap().to_bits(),
            DRAG_BY_SPEED[0].to_bits()
        );
        // Between Mach 6.3 and Mach 8.8: a straight line from 1.6082 to the read by speed.
        let got = drag_at(&capsule, 1700.0, 7.55).unwrap();
        let want: f64 = 1.6082 + (1.6268 - 1.6082) * ((7.55 - 6.3) / (8.8 - 6.3));
        assert_eq!(got.to_bits(), want.to_bits());
        // The two ends of the line meet their tables.
        assert_eq!(
            drag_at(&capsule, 1700.0, 6.3).unwrap().to_bits(),
            1.6082_f64.to_bits()
        );
        assert_eq!(
            drag_at(&capsule, 1700.0, 8.8).unwrap().to_bits(),
            1.6268_f64.to_bits()
        );
        // Outside the tables there is no drag.
        assert_eq!(
            drag_at(&capsule, 300.0, 0.89),
            Err(StepReject::MachOutsideCard)
        );
        assert_eq!(
            drag_at(&capsule, 5850.1, 25.0),
            Err(StepReject::AirSpeedOutsideCard)
        );
        assert!(between_entries(&DRAG_MACH, &DRAG_BY_MACH, 6.31).is_none());
        assert!(between_entries(&DRAG_SPEED_KM_S, &DRAG_BY_SPEED, 1.79).is_none());
    }

    #[test]
    fn free_fall_conserves_energy_and_angular_momentum_to_rounding() {
        let capsule = CapsuleInsight::entry_capsule();
        let mut vacuum = capsule;
        vacuum.air_density_kg_m3 = [0.0; KNOTS];
        // Without air the capsule leaves the corridor the compared runs flew;
        // the copy's corridor is opened so that only the integration is tested.
        vacuum.corridor_air_speed_low_m_s = [0.0; CORRIDOR_KNOTS];
        vacuum.corridor_air_speed_high_m_s = [1.0e6; CORRIDOR_KNOTS];
        vacuum.corridor_path_sine_low = [-1.0; CORRIDOR_KNOTS];
        vacuum.corridor_path_sine_high = [1.0; CORRIDOR_KNOTS];
        assert!(!vacuum.is_valid());
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let start = recorded(&state);
        let energy_0 = rotating_frame_energy_j_kg(&capsule, start[0], start[1], start[2]);
        let fixed_0 = fixed_frame_energy_j_kg(&capsule, start);
        let momentum_0 = start[0] * start[2];
        let mut worst_energy = 0.0;
        let mut worst_fixed = 0.0;
        let mut worst_momentum = 0.0;
        let mut n = 0u32;
        while n < 1_000 {
            // The real capsule's validation would refuse this state as off
            // the corridor, so the step is integrated and booked by the
            // vacuum copy alone.
            let predicted = integrate(&vacuum, recorded(&state), 0.0, DT).unwrap();
            commit(&vacuum, &mut state, &mut frame, 0.0, DT, predicted).unwrap();
            n += 1;
            let y = recorded(&state);
            assert_eq!(
                y[4].to_bits(),
                0.0_f64.to_bits(),
                "step {n} booked a drag loss"
            );
            let drift = abs_f64(rotating_frame_energy_j_kg(&capsule, y[0], y[1], y[2]) - energy_0)
                / ENERGY_SCALE_J_KG;
            if drift > worst_energy {
                worst_energy = drift;
            }
            let drift = abs_f64(fixed_frame_energy_j_kg(&capsule, y) - fixed_0) / ENERGY_SCALE_J_KG;
            if drift > worst_fixed {
                worst_fixed = drift;
            }
            let drift = abs_f64(y[0] * y[2] / momentum_0 - 1.0);
            if drift > worst_momentum {
                worst_momentum = drift;
            }
        }
        assert!(
            worst_energy < 1.0e-13,
            "rotating-frame energy drift {worst_energy}"
        );
        assert!(
            worst_fixed < 1.0e-13,
            "fixed-frame energy drift {worst_fixed}"
        );
        assert!(
            worst_momentum < 1.0e-13,
            "angular momentum drift {worst_momentum}"
        );
        assert_eq!(state.load_g.to_bits(), 0.0_f64.to_bits());
        // The model in table form after the same 1,000 steps with the air removed.
        assert_eq!(state.radius_m.to_bits(), 3430819.9449372436_f64.to_bits());
        assert_eq!(
            state.radial_speed_m_s.to_bits(),
            (-616.1846535295363_f64).to_bits()
        );
        assert_eq!(
            state.transverse_speed_m_s.to_bits(),
            5783.75949479249_f64.to_bits()
        );
        assert_eq!(state.angle_rad.to_bits(), 0.16467164510948515_f64.to_bits());
        proof_intact(&frame);
    }

    #[test]
    fn the_entry_without_lift_reproduces_the_reference_states_bit_for_bit() {
        check_reference(NO_LIFT, &REFERENCE_NO_LIFT);
    }

    #[test]
    fn the_entry_with_the_flights_lift_reproduces_the_reference_states_bit_for_bit() {
        check_reference(FLIGHT_LIFT, &REFERENCE_FLIGHT_LIFT);
    }

    #[test]
    fn the_peak_load_falls_where_the_reference_puts_it() {
        let capsule = CapsuleInsight::entry_capsule();
        for (load, peak_step, peak_g) in [
            (NO_LIFT, PEAK_STEP_NO_LIFT, 7.600699812494663_f64),
            (FLIGHT_LIFT, PEAK_STEP_FLIGHT_LIFT, 7.951853813798471_f64),
        ] {
            let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
            let mut frame = blank_frame();
            let mut best = 0.0;
            let mut best_step = 0u32;
            let mut worst_residual = 0.0;
            let mut n = 0u32;
            while step(&capsule, &mut state, &mut frame, load, DT).is_ok() {
                n += 1;
                if state.load_g > best {
                    best = state.load_g;
                    best_step = n;
                }
                if state.residual_conservation_error > worst_residual {
                    worst_residual = state.residual_conservation_error;
                }
            }
            assert_eq!(best_step, peak_step);
            assert_eq!(best.to_bits(), peak_g.to_bits());
            // Over the whole entry the residual stays near a thousandth of its limit.
            assert!(worst_residual < 2.0e-12, "{worst_residual}");
            proof_intact(&frame);
        }
    }

    #[test]
    fn the_floor_is_reached_by_name_with_nothing_clamped() {
        let capsule = CapsuleInsight::entry_capsule();
        for (load, last) in [
            (NO_LIFT, LAST_STEP_NO_LIFT),
            (FLIGHT_LIFT, LAST_STEP_FLIGHT_LIFT),
        ] {
            let (mut state, mut frame) = run_to(&capsule, load, last);
            // Both runs stop at the compared Mach floor, above the compared
            // altitude floor and far above the bottom of the air tables.
            assert!(state.mach >= COMPARED_MACH_FLOOR && state.mach < COMPARED_MACH_FLOOR + 0.003);
            assert!(state.altitude_m >= COMPARED_ALTITUDE_FLOOR_M);
            let saved = state_bits(&state);
            let held = frame.to_bytes();
            assert_eq!(
                step(&capsule, &mut state, &mut frame, load, DT),
                Err(StepReject::PastComparedFlight)
            );
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            proof_intact(&frame);
        }
        // With the flight's lift the run lands past the flight's deployment
        // time and past the flight's speed there, so both are scored on landed
        // steps. With no lift it stops above the flight's deployment altitude:
        // the time to reach it is no longer a landed step.
        let (state, _) = run_to(&capsule, FLIGHT_LIFT, LAST_STEP_FLIGHT_LIFT);
        assert!(state.t_s > FLIGHT_DEPLOYMENT_T_S);
        assert!(state.air_speed_m_s < FLIGHT_DEPLOYMENT_AIR_SPEED_M_S);
        assert!(state.altitude_m < FLIGHT_DEPLOYMENT_ALTITUDE_M);
        let (state, _) = run_to(&capsule, NO_LIFT, LAST_STEP_NO_LIFT);
        assert!(state.air_speed_m_s < FLIGHT_DEPLOYMENT_AIR_SPEED_M_S);
        assert!(state.altitude_m > FLIGHT_DEPLOYMENT_ALTITUDE_M);
        // With the most lift toward Mars the altitude floor comes first.
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let down = CapsuleLoad {
            lift_over_drag: -LIFT_OVER_DRAG_LIMIT,
        };
        let refusal = loop {
            if let Err(reject) = step(&capsule, &mut state, &mut frame, down, DT) {
                break reject;
            }
        };
        assert_eq!(refusal, StepReject::PastComparedFlight);
        assert!(state.altitude_m >= COMPARED_ALTITUDE_FLOOR_M);
        assert!(state.altitude_m < COMPARED_ALTITUDE_FLOOR_M + 25.0);
        assert!(state.mach > 1.7);
    }

    #[test]
    fn explicit_euler_is_refused_on_the_first_step() {
        let capsule = CapsuleInsight::entry_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let got = step_euler(&capsule, &mut state, &mut frame, DT);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        // How far over the limit: +1.05e-8 against 1e-9.
        let y = recorded(&state);
        let trial = axpy(y, DT, rates(&capsule, y, 0.0).unwrap());
        let defect = rotating_frame_energy_j_kg(&capsule, trial[0], trial[1], trial[2])
            - state.energy_anchor_j_kg
            + trial[4];
        let residual = defect / ENERGY_SCALE_J_KG;
        assert_eq!(
            residual.to_bits(),
            1.0514794257379503e-08_f64.to_bits(),
            "{residual}"
        );

        // The honest step from the same state lands.
        step(&capsule, &mut state, &mut frame, NO_LIFT, DT).unwrap();
        assert!(state.residual_conservation_error < 1.0e-15);
        proof_intact(&frame);
    }

    #[test]
    fn explicit_euler_at_a_fine_step_adds_up_to_a_refusal() {
        let capsule = CapsuleInsight::entry_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut landed = 0u32;
        let mut last = 0.0;
        let refusal = loop {
            match step_euler(&capsule, &mut state, &mut frame, 0.01) {
                Ok(()) => {
                    landed += 1;
                    assert!(state.energy_defect_j_kg > last, "the defect did not grow");
                    last = state.energy_defect_j_kg;
                }
                Err(reject) => break reject,
            }
            assert!(landed < 1_000, "explicit Euler was never refused");
        };
        assert_eq!(refusal, StepReject::SpontaneousGeneration);
        assert_eq!(landed, 9);
        proof_intact(&frame);
    }

    #[test]
    fn a_ledger_that_forgets_the_air_turns_is_refused() {
        let capsule = CapsuleInsight::entry_capsule();
        // The same commit, read from a copy whose air does not turn, anchored
        // at the fixed-frame energy. The step itself is the honest one.
        let mut still = capsule;
        still.air_rotation_rate_rad_s = 0.0;
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let y = recorded(&state);
        state.energy_anchor_j_kg = fixed_frame_energy_j_kg(&capsule, y);
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let got = step_with_copies(&capsule, &capsule, &still, &mut state, &mut frame, DT);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        // How far: −5.4e-9 after one step of 0.1 s.
        let trial = integrate(&capsule, y, 0.0, DT).unwrap();
        let defect = fixed_frame_energy_j_kg(&capsule, trial) - state.energy_anchor_j_kg + trial[4];
        let residual = defect / ENERGY_SCALE_J_KG;
        assert_eq!(
            residual.to_bits(),
            (-5.376715904211127e-09_f64).to_bits(),
            "{residual}"
        );

        // The ledger that counts the turning air lands the same step.
        let mut honest = CapsuleState::try_entry_interface(&capsule).unwrap();
        step(&capsule, &mut honest, &mut frame, NO_LIFT, DT).unwrap();
        assert_eq!(honest.radius_m.to_bits(), trial[0].to_bits());
        assert!(honest.residual_conservation_error < 1.0e-15);
        proof_intact(&frame);
    }

    #[test]
    fn a_biased_drag_loss_is_refused_both_ways() {
        let capsule = CapsuleInsight::entry_capsule();
        let (state, mut frame) = run_to(&capsule, NO_LIFT, 1);
        let held = frame.to_bytes();
        let extra = 1.0e-3 * ENERGY_SCALE_J_KG;
        for (bias, want) in [
            (extra, StepReject::SpontaneousGeneration),
            (-extra, StepReject::FirstLawBreach),
            (
                2.0e-9 * ENERGY_SCALE_J_KG,
                StepReject::SpontaneousGeneration,
            ),
            (-2.0e-9 * ENERGY_SCALE_J_KG, StepReject::FirstLawBreach),
        ] {
            let mut trial = state;
            let saved = state_bits(&trial);
            assert_eq!(
                step_bias_drag_loss(&capsule, &mut trial, &mut frame, DT, bias),
                Err(want)
            );
            assert_eq!(state_bits(&trial), saved);
            assert_eq!(frame.to_bytes(), held);
        }
        // Half the limit lands.
        let mut trial = state;
        step_bias_drag_loss(
            &capsule,
            &mut trial,
            &mut frame,
            DT,
            0.5 * RESIDUAL_CONSERVATION_LIMIT * ENERGY_SCALE_J_KG,
        )
        .unwrap();
        proof_intact(&frame);
    }

    #[test]
    fn lift_laid_along_the_path_is_refused_and_lift_across_it_lands() {
        let capsule = CapsuleInsight::entry_capsule();
        let (state, mut frame) = run_to(&capsule, NO_LIFT, 900);
        let held = frame.to_bytes();
        let y = recorded(&state);
        // One RK4 step with a force of 0.031 of drag pushing along the path.
        let f = |y: [f64; 5]| rates_with_lift_along_the_path(&capsule, y, 0.031);
        let k1 = f(y);
        let k2 = f(axpy(y, DT * 0.5, k1));
        let k3 = f(axpy(y, DT * 0.5, k2));
        let k4 = f(axpy(y, DT, k3));
        let mut pushed = y;
        let mut index = 0;
        while index < 5 {
            pushed[index] +=
                (DT / 6.0) * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
            index += 1;
        }
        let mut trial = state;
        let saved = state_bits(&trial);
        assert_eq!(
            commit(&capsule, &mut trial, &mut frame, 0.031, DT, pushed),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(state_bits(&trial), saved);
        assert_eq!(frame.to_bytes(), held);
        let defect = rotating_frame_energy_j_kg(&capsule, pushed[0], pushed[1], pushed[2])
            - state.energy_anchor_j_kg
            + pushed[4];
        // 5.3e-5 of the entry energy in one step, against a limit of 1e-9.
        assert!(defect / ENERGY_SCALE_J_KG > 5.0e-5 && defect / ENERGY_SCALE_J_KG < 5.6e-5);

        // The same size of lift across the path, either way, lands and closes the ledger.
        for lift in [0.031, -0.031, LIFT_OVER_DRAG_LIMIT, -LIFT_OVER_DRAG_LIMIT] {
            let mut trial = state;
            step(
                &capsule,
                &mut trial,
                &mut frame,
                CapsuleLoad {
                    lift_over_drag: lift,
                },
                DT,
            )
            .unwrap();
            assert!(trial.residual_conservation_error < 2.0e-12);
            assert_eq!(frame.flags & FLAG_LIFT_ACTING, FLAG_LIFT_ACTING);
            // Lift does not change the drag loss booked in the step to first order.
            assert!(abs_f64(trial.drag_loss_j_kg - state.drag_loss_j_kg) > 0.0);
        }
        proof_intact(&frame);
    }

    #[test]
    fn lift_away_from_mars_holds_the_capsule_up_and_lift_toward_mars_brings_it_down() {
        let capsule = CapsuleInsight::entry_capsule();
        let (none, _) = run_to(&capsule, NO_LIFT, 1200);
        let (up, _) = run_to(
            &capsule,
            CapsuleLoad {
                lift_over_drag: 0.05,
            },
            1200,
        );
        let (down, _) = run_to(
            &capsule,
            CapsuleLoad {
                lift_over_drag: -0.05,
            },
            1200,
        );
        assert!(up.altitude_m > none.altitude_m + 1_000.0);
        assert!(down.altitude_m < none.altitude_m - 1_000.0);
        assert_eq!(up.altitude_m.to_bits(), 31461.632556601893_f64.to_bits());
        assert_eq!(down.altitude_m.to_bits(), 26771.86621048767_f64.to_bits());
    }

    #[test]
    fn every_edge_of_the_card_is_refused_by_name() {
        let capsule = CapsuleInsight::entry_capsule();
        let entry = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let lift = |value: f64| CapsuleLoad {
            lift_over_drag: value,
        };

        let cases: [(f64, CapsuleLoad, StepReject); 12] = [
            (f64::NAN, NO_LIFT, StepReject::NonFiniteInput),
            (f64::INFINITY, NO_LIFT, StepReject::NonFiniteInput),
            (f64::NEG_INFINITY, NO_LIFT, StepReject::NonFiniteInput),
            (DT, lift(f64::NAN), StepReject::NonFiniteInput),
            (DT, lift(f64::INFINITY), StepReject::NonFiniteInput),
            (0.0, NO_LIFT, StepReject::NonPositiveDt),
            (-0.0, NO_LIFT, StepReject::NonPositiveDt),
            (-DT, NO_LIFT, StepReject::NonPositiveDt),
            (DT, lift(0.080_000_1), StepReject::LiftOutsideCard),
            (DT, lift(-0.080_000_1), StepReject::LiftOutsideCard),
            (DT, lift(1.0), StepReject::LiftOutsideCard),
            // A step 1,200 times the ceiling stays inside the card at every
            // stage and is stopped by the ledger.
            (120.0, NO_LIFT, StepReject::SpontaneousGeneration),
        ];
        let mut index = 0;
        while index < cases.len() {
            let (dt, load, want) = cases[index];
            let mut state = entry;
            let saved = state_bits(&state);
            assert_eq!(
                step(&capsule, &mut state, &mut frame, load, dt),
                Err(want),
                "case {index}"
            );
            assert_eq!(state_bits(&state), saved, "case {index}");
            assert_eq!(frame.to_bytes(), held, "case {index}");
            index += 1;
        }
        // The limit itself is inside the card.
        let mut state = entry;
        step(
            &capsule,
            &mut state,
            &mut frame,
            lift(LIFT_OVER_DRAG_LIMIT),
            DT,
        )
        .unwrap();
        let mut state = entry;
        step(
            &capsule,
            &mut state,
            &mut frame,
            lift(-LIFT_OVER_DRAG_LIMIT),
            DT,
        )
        .unwrap();
        let mut frame = blank_frame();

        // A non-finite number in the state.
        let mut state = entry;
        state.radial_speed_m_s = f64::NAN;
        let saved = state_bits(&state);
        assert_eq!(
            step(&capsule, &mut state, &mut frame, NO_LIFT, DT),
            Err(StepReject::NonFiniteState)
        );
        assert_eq!(state_bits(&state), saved);
        let mut state = entry;
        state.drag_loss_j_kg = f64::INFINITY;
        assert_eq!(
            step(&capsule, &mut state, &mut frame, NO_LIFT, DT),
            Err(StepReject::NonFiniteState)
        );

        // States outside the card cannot be made, and a state forced there is refused.
        assert!(
            CapsuleState::try_state(&capsule, DATUM_RADIUS_M + 128_000.1, -1000.0, 5000.0)
                .is_none()
        );
        assert!(
            CapsuleState::try_state(&capsule, DATUM_RADIUS_M + 3_999.9, -100.0, 600.0).is_none()
        );
        assert!(
            CapsuleState::try_state(&capsule, DATUM_RADIUS_M + 10_000.0, -50.0, 400.0).is_none()
        );
        assert!(
            CapsuleState::try_state(&capsule, DATUM_RADIUS_M + 60_000.0, -1000.0, 6200.0).is_none()
        );
        assert!(CapsuleState::try_state(&capsule, f64::NAN, -1000.0, 5000.0).is_none());
        assert!(
            CapsuleState::try_state(&capsule, DATUM_RADIUS_M + 60_000.0, f64::INFINITY, 5000.0)
                .is_none()
        );
        // The top of the air tables and their bottom are not on the card:
        // the one is above the corridor, the other below the compared floor.
        assert_eq!(
            CapsuleState::start(
                &capsule,
                DATUM_RADIUS_M + ALTITUDE_CEILING_M,
                -1000.0,
                5000.0
            ),
            Err(StepReject::OffComparedCorridor)
        );
        assert_eq!(
            CapsuleState::start(&capsule, DATUM_RADIUS_M + ALTITUDE_FLOOR_M, -100.0, 600.0),
            Err(StepReject::PastComparedFlight)
        );

        let forced: [(f64, f64, f64, StepReject); 7] = [
            (
                DATUM_RADIUS_M + 128_000.1,
                -1000.0,
                5000.0,
                StepReject::AltitudeOutsideCard,
            ),
            (
                DATUM_RADIUS_M + 3_999.9,
                -100.0,
                600.0,
                StepReject::AltitudeOutsideCard,
            ),
            // 229 m/s through air whose speed of sound is 235 m/s.
            (
                DATUM_RADIUS_M + 10_000.0,
                -50.0,
                458.0,
                StepReject::MachOutsideCard,
            ),
            (
                DATUM_RADIUS_M + 60_000.0,
                -1000.0,
                6200.0,
                StepReject::AirSpeedOutsideCard,
            ),
            // Mach 2.5 at 5 km, and Mach 1.2 at 9 km.
            (
                DATUM_RADIUS_M + 5_000.0,
                -300.0,
                764.0,
                StepReject::PastComparedFlight,
            ),
            (
                DATUM_RADIUS_M + 9_000.0,
                -150.0,
                494.0,
                StepReject::PastComparedFlight,
            ),
            // Climbing at 20 km.
            (
                DATUM_RADIUS_M + 20_000.0,
                100.0,
                1_400.0,
                StepReject::OffComparedCorridor,
            ),
        ];
        let mut index = 0;
        while index < forced.len() {
            let (radius, radial, transverse, want) = forced[index];
            let mut state = entry;
            state.radius_m = radius;
            state.radial_speed_m_s = radial;
            state.transverse_speed_m_s = transverse;
            let saved = state_bits(&state);
            let held = frame.to_bytes();
            assert_eq!(
                step(&capsule, &mut state, &mut frame, NO_LIFT, DT),
                Err(want),
                "forced state {index}"
            );
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            index += 1;
        }

        // Each forced state is refused at the door by the same name.
        let mut index = 0;
        while index < forced.len() {
            let (radius, radial, transverse, want) = forced[index];
            assert_eq!(
                CapsuleState::start(&capsule, radius, radial, transverse),
                Err(want),
                "start {index}"
            );
            index += 1;
        }
        assert_eq!(
            CapsuleState::start(&capsule, f64::NAN, -1000.0, 5000.0),
            Err(StepReject::NonFiniteState)
        );
        proof_intact(&frame);
        let _ = (
            AIR_SPEED_CEILING_M_S,
            CAPSULE_MASS_KG,
            REFERENCE_AREA_M2,
            STANDARD_GRAVITY_M_S2,
        );
    }

    #[test]
    fn every_compared_run_stays_in_the_corridor_and_a_start_off_it_is_refused() {
        let capsule = CapsuleInsight::entry_capsule();
        // Held lifts, some between the values the corridor was built from,
        // and the lift changed at 150 s: each run lands until the compared
        // floor and never leaves the corridor.
        for (first, then) in [
            (0.08, 0.08),
            (0.0399, 0.0399),
            (0.0013, 0.0013),
            (-0.0611, -0.0611),
            (-0.08, -0.08),
            (-0.031, 0.08),
            (-0.031, -0.08),
            (0.0, -0.08),
            (0.0, 0.0412),
        ] {
            let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
            let mut frame = blank_frame();
            let mut n = 0u32;
            let refusal = loop {
                let lift = if n < 1500 { first } else { then };
                match step(&capsule, &mut state, &mut frame, CapsuleLoad { lift_over_drag: lift }, DT) {
                    Ok(()) => n += 1,
                    Err(reject) => break reject,
                }
            };
            assert_eq!(refusal, StepReject::PastComparedFlight, "{first} then {then}");
            assert!(n > 1800, "{first} then {then}: {n}");
        }

        // A restart from a returned state lands, and lands where the run does.
        let (mut state, mut frame) = run_to(&capsule, FLIGHT_LIFT, 1500);
        let mut restarted = CapsuleState::try_state(
            &capsule,
            state.radius_m,
            state.radial_speed_m_s,
            state.transverse_speed_m_s,
        )
        .unwrap();
        let mut other = blank_frame();
        for _ in 0..600 {
            step(&capsule, &mut state, &mut frame, FLIGHT_LIFT, DT).unwrap();
            step(&capsule, &mut restarted, &mut other, FLIGHT_LIFT, DT).unwrap();
        }
        assert_eq!(restarted.radius_m.to_bits(), state.radius_m.to_bits());
        assert_eq!(restarted.radial_speed_m_s.to_bits(), state.radial_speed_m_s.to_bits());
        assert_eq!(
            restarted.transverse_speed_m_s.to_bits(),
            state.transverse_speed_m_s.to_bits()
        );

        // Two starts outside the compared flight: climbing at the old 4 km
        // floor, and 5804 m/s through the air at 100 km, above any speed flown.
        assert_eq!(
            CapsuleState::start(&capsule, DATUM_RADIUS_M + 4_000.0, 50.0, 600.0),
            Err(StepReject::PastComparedFlight)
        );
        assert_eq!(
            CapsuleState::start(&capsule, DATUM_RADIUS_M + 100_000.0, -1000.0, 5954.0),
            Err(StepReject::OffComparedCorridor)
        );
        // Above the top knot, on the entry's own path angle and speed.
        let above_top = phase_at(&capsule, CORRIDOR_TOP_M + 1.0, 5523.4, -0.2176);
        assert_eq!(
            CapsuleState::start(&capsule, above_top.0, above_top.1, above_top.2),
            Err(StepReject::OffComparedCorridor)
        );
        // At knots and between them: the middle lands, and 1 m/s or 0.001 of
        // sine past each bound is refused, incoming and as a start.
        for altitude in [
            CORRIDOR_TOP_M,
            126_000.0,
            100_250.0,
            60_000.0,
            31_000.0,
            15_100.0,
            8_000.0,
            COMPARED_ALTITUDE_FLOOR_M,
        ] {
            let [low, high, sine_low, sine_high] = corridor_at(altitude);
            let middle = phase_at(&capsule, altitude, 0.5 * (low + high), 0.5 * (sine_low + sine_high));
            let landed = CapsuleState::start(&capsule, middle.0, middle.1, middle.2);
            assert!(landed.is_ok(), "{altitude}: {landed:?}");
            let mut state = landed.unwrap();
            let mut frame = blank_frame();
            assert!(state.mach >= COMPARED_MACH_FLOOR);
            // Going down, a step lands from every altitude but the floor.
            let stepped = step(&capsule, &mut state, &mut frame, NO_LIFT, DT / 1024.0);
            if altitude > COMPARED_ALTITUDE_FLOOR_M {
                assert_eq!(stepped, Ok(()), "{altitude}");
            } else {
                assert_eq!(stepped, Err(StepReject::PastComparedFlight));
            }
            for (speed, sine) in [
                (low - 1.0, 0.5 * (sine_low + sine_high)),
                (high + 1.0, 0.5 * (sine_low + sine_high)),
                (0.5 * (low + high), sine_low - 0.001),
                (0.5 * (low + high), sine_high + 0.001),
            ] {
                let off = phase_at(&capsule, altitude, speed, sine);
                let started = CapsuleState::start(&capsule, off.0, off.1, off.2);
                let want = if speed / 239.0 < COMPARED_MACH_FLOOR {
                    StepReject::PastComparedFlight
                } else {
                    StepReject::OffComparedCorridor
                };
                assert_eq!(started.err(), Some(want), "{altitude} {speed} {sine}");
                let mut forced = CapsuleState::try_entry_interface(&capsule).unwrap();
                forced.radius_m = off.0;
                forced.radial_speed_m_s = off.1;
                forced.transverse_speed_m_s = off.2;
                assert_eq!(
                    step(&capsule, &mut forced, &mut frame, NO_LIFT, DT),
                    Err(want),
                    "{altitude} {speed} {sine}"
                );
            }
        }
        // The compared altitude floor itself is on the card; the next number
        // under it is not.
        let [low, high, sine_low, sine_high] = corridor_at(COMPARED_ALTITUDE_FLOOR_M);
        let on = phase_at(&capsule, COMPARED_ALTITUDE_FLOOR_M, 0.5 * (low + high), 0.5 * (sine_low + sine_high));
        let state = CapsuleState::start(&capsule, on.0, on.1, on.2).unwrap();
        assert_eq!(state.altitude_m.to_bits(), COMPARED_ALTITUDE_FLOOR_M.to_bits());
        let under = f64::from_bits(on.0.to_bits() - 1);
        assert_eq!(
            CapsuleState::start(&capsule, under, on.1, on.2),
            Err(StepReject::PastComparedFlight)
        );
    }

    #[test]
    fn flags_are_only_flags() {
        let capsule = CapsuleInsight::entry_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        assert_eq!(flags_of(&state), 0);
        let mut frame = blank_frame();
        let mut n = 0u32;
        while step(&capsule, &mut state, &mut frame, NO_LIFT, DT).is_ok() {
            n += 1;
            assert_eq!(frame.flags, 0);
        }
        assert_eq!(n, LAST_STEP_NO_LIFT);
        // With lift the other flag rides every frame, and a step back at zero lift clears it.
        let (mut state, mut frame) = run_to(&capsule, FLIGHT_LIFT, 10);
        assert_eq!(frame.flags, FLAG_LIFT_ACTING);
        step(&capsule, &mut state, &mut frame, NO_LIFT, DT).unwrap();
        assert_eq!(frame.flags, 0);
        assert!(!state.lift_acting);
        proof_intact(&frame);
    }

    #[test]
    fn a_frame_is_not_a_state() {
        let capsule = CapsuleInsight::entry_capsule();
        let (state, frame) = run_to(&capsule, NO_LIFT, 1047);
        // The frame holds 32-bit numbers. At 30 km a 32-bit altitude moves in
        // steps of 2 mm, and the frame carries no transverse speed at all.
        assert!(f64::from(frame.pos[0]) != state.altitude_m);
        assert!(abs_f64(f64::from(frame.pos[0]) - state.altitude_m) < 2.0e-3);
        assert!(abs_f64(f64::from(frame.vel[0]) - state.air_speed_m_s) < 3.0e-4);
        assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
        // A state rebuilt from the frame's altitude and speeds is another state.
        let rebuilt = CapsuleState::try_state(
            &capsule,
            DATUM_RADIUS_M + f64::from(frame.pos[0]),
            f64::from(frame.vel[1]),
            state.transverse_speed_m_s,
        )
        .unwrap();
        assert!(rebuilt.radius_m.to_bits() != state.radius_m.to_bits());
        assert_eq!(rebuilt.t_s.to_bits(), 0.0_f64.to_bits());
        assert_eq!(rebuilt.drag_loss_j_kg.to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn the_frame_carries_the_documented_slots() {
        let capsule = CapsuleInsight::entry_capsule();
        let (state, frame) = run_to(&capsule, FLIGHT_LIFT, 1065);
        assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
        assert_eq!(frame.pos[0].to_bits(), (state.altitude_m as f32).to_bits());
        assert_eq!(
            frame.pos[1].to_bits(),
            (state.ground_distance_m as f32).to_bits()
        );
        assert_eq!(frame.pos[2].to_bits(), (state.mach as f32).to_bits());
        assert_eq!(
            frame.vel[0].to_bits(),
            (state.air_speed_m_s as f32).to_bits()
        );
        assert_eq!(
            frame.vel[1].to_bits(),
            (state.radial_speed_m_s as f32).to_bits()
        );
        assert_eq!(
            frame.vel[2].to_bits(),
            (state.dynamic_pressure_pa as f32).to_bits()
        );
        assert_eq!(
            frame.force_torque.to_bits(),
            (state.load_g as f32).to_bits()
        );
        assert_eq!(
            frame.residual.to_bits(),
            (state.residual_conservation_error as f32).to_bits()
        );
        assert_eq!(frame.flags, FLAG_LIFT_ACTING);
        // The load is drag and lift together.
        let drag_g = state.load_g / (1.0_f64 + 0.031 * 0.031).sqrt();
        assert!(state.load_g > drag_g && state.load_g - drag_g < 0.005);
        proof_intact(&frame);
    }

    #[test]
    fn the_proof_bytes_are_never_touched() {
        let capsule = CapsuleInsight::entry_capsule();
        let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
        let mut frame = blank_frame();
        let mut index = 0;
        while index < 16 {
            frame.proof[index] = (index as u8).wrapping_mul(17).wrapping_add(3);
            index += 1;
        }
        let proof = frame.proof;
        let mut n = 0;
        while n < 400 {
            step(&capsule, &mut state, &mut frame, FLIGHT_LIFT, DT).unwrap();
            assert_eq!(frame.proof, proof);
            n += 1;
        }
        assert_eq!(
            step(&capsule, &mut state, &mut frame, FLIGHT_LIFT, f64::NAN),
            Err(StepReject::NonFiniteInput)
        );
        assert_eq!(frame.proof, proof);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let capsule = CapsuleInsight::entry_capsule();
        let floor = capsule.floor_dt_s().unwrap();
        let (mut state, mut frame) = run_to(&capsule, NO_LIFT, 1047);
        let before = recorded(&state);
        let t = state.t_s;
        step(&capsule, &mut state, &mut frame, NO_LIFT, floor).unwrap();
        let after = recorded(&state);
        assert!(state.t_s > t);
        let mut index = 0;
        while index < 5 {
            assert!(
                after[index].to_bits() != before[index].to_bits(),
                "place {index} did not move"
            );
            index += 1;
        }
        proof_intact(&frame);
    }
}
