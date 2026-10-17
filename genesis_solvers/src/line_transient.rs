//! The transient of a liquid line between a supply held at one pressure and
//! a valve that closes: the method of characteristics on a fixed grid, with
//! free gas and vapour lumped at the grid's nodes. Not a solver: the solver
//! `water_hammer_oil_line` runs it on its own line, and its tests run it on
//! the copper rig it is compared with.
//!
//! The line. One or more pipes in series, each cut into sections of equal
//! length. In each section the liquid obeys the acoustic line equations
//!
//! ```text
//! rho dV/dt + dp/dx = -F(V),       F = rho f(Re) V |V| / (2 D)
//! dp/dt + rho a^2 dV/dx = 0,       1/(rho a^2) = 1/K + D psi / (E e)
//! ```
//!
//! with `a` the wave speed of the liquid in that pipe, its own
//! compressibility and the stretch of the wall, and `f` Churchill's (1977)
//! friction factor, one formula for laminar, transitional and turbulent
//! flow; below a Reynolds number of about 1.6e-25, where the formula's
//! laminar term passes the 64-bit range, its laminar limit, Poiseuille's
//! `64/Re`, written as the force `32 mu V / D^2`. Along `dx/dt = +a` the quantity `C+ = p + Z Q` and along
//! `dx/dt = -a` the quantity `C- = p - Z Q` are carried, `Z = rho a / A`,
//! changed only by friction. The time step is the time a wave takes to cross
//! one section, the same in every pipe: a pipe's sections are its length
//! over its count, and its wave speed is taken as that length over the time
//! step (a pipe of a line whose wave speeds do not divide evenly is given
//! the wave speed that does, and the run says by how much it moved). Nothing
//! is interpolated, so a wave crosses the line with no numerical damping.
//!
//! Friction. In flight across one section both waves lose the same
//! pressure, `delta = dx F(V_mid)`, where `V_mid` is the section's velocity
//! halfway through the loss: `Z Q_mid = (C+ - C-)/2 - delta/2`. The fixed
//! point is iterated to its last bit. The energy friction takes is then
//! exactly `F(V_mid) dx A V_mid dt`, force times velocity, never negative,
//! and the two waves together carry the same volume as before.
//!
//! The nodes. Where two sections meet, and at the valve, the node holds a
//! cavity: the free gas lumped from the liquid around it (the discrete gas
//! cavity model of Wylie 1984), isothermal, `p V_gas = M`, and, where the
//! pressure would fall below the vapour pressure, a vapour cavity (the
//! discrete vapour cavity model) at the vapour pressure. The stored energy
//! of a node, counted from the line at rest at the supply pressure, is
//!
//! ```text
//! U(V) = M ln(V_ref / V) + p_s (V - V_ref)            V <= V* = M / p_v
//! U(V) = U(V*) + (p_s - p_v) (V - V*)                 V >  V*
//! ```
//!
//! with `V_ref = M / p_s` and `V` the cavity's whole volume, gas and vapour.
//! Over one step a node takes the pressure of its cavity at the step's end,
//! `V_new = V_old - (Q_in - Q_out) dt` with the flows at that pressure: the
//! weighting of one in Wylie's model. Because `U` is convex the work the
//! waves do on the cavity is never less than the cavity's gain of stored
//! energy; the difference, `cavity_loss_j`, is the damping this backward
//! step gives at the grid's own scale, where a node's lumped gas would
//! otherwise ring, and it holds the last step of each closing vapour cavity:
//! the liquid columns meet and the pressure rises above the vapour pressure
//! with no volume left to do work on. A node that gave energy back would be
//! stopped by name: the test allows the rounding of the node's own terms
//! (its pressure times its volumes, its energy's terms) and what its root
//! leaves unsettled (a lump's Newton root is settled to a part in 1e12 of
//! its volume, and its energy is taken there), and nothing else. The new
//! volume is found by false position with bisection on a function that
//! rises strictly, so it has one root, and past its passes by halving to
//! the bracket's last bit. A gas so small that the rounding of the flows
//! takes the volume they leave to zero or below keeps the root's own
//! volume, which it is to that rounding. A node at its state at rest at the
//! supply pressure whose flows balance at that pressure is its step's root
//! exactly and keeps it: a line at rest stays at rest to the bit.
//!
//! The ledgers. Energy is counted from the line at rest at the supply
//! pressure: the waves in flight carry `dt (C+^2 + C-^2) / (4 Z)` per
//! section (kinetic `dt (C+ - C-)^2 / (8 Z)`, elastic `dt (C+ + C-)^2 /
//! (8 Z)`, the elastic part split into the liquid's compression and the
//! wall's stretch by their shares of the compliance), the nodes their `U`.
//! The supply then does no work; the valve takes `(p_v - p_s) Q_v dt` each
//! step; friction takes its dissipation, and the nodes their cavity loss. Volume: the waves carry
//! `dt (C+ + C-) / (2 Z)` per section, the liquid stored by compression
//! over the line at the supply pressure, and the cavities take their volume
//! from the liquid; the stored volume less the cavities' rises by what the
//! supply adds, `Q_in dt`, and falls by what the valve lets out, `Q_v dt`. Both sums are
//! closed at every step from the fields, against the boundary and loss
//! terms summed step by step. Each is judged over a scale: the physical one
//! (the line's volume times its kinetic and compression energy, or its
//! compression volume and gas), or, where that is smaller, [`SCALE_FLOOR`]
//! of the sizes of the terms the ledger adds at that step and
//! [`RUN_FLOOR`] of the lumped nodes' terms over the run, below which it
//! would judge rounding, not physics: a line creeping near its vapour
//! pressure shrinks the first, not the second.

use genesis_kernel::math;
/// Standard atmosphere, pascals: the pressure at which a free gas fraction
/// is stated.
pub const ATMOSPHERE_PA: f64 = 101_325.0;

/// The largest number of passes of the friction fixed point. It contracts
/// by `dx F'(V) / (2 rho a)`, under 1e-3 on every line served, so it settles
/// in a handful.
pub const MAX_FRICTION_PASSES: u32 = 60;
/// The friction of a section is settled when a pass moves it by no more
/// than this fraction.
pub const FRICTION_SETTLED: f64 = 1.0e-13;
/// The largest number of passes of a node's root search.
pub const MAX_NODE_PASSES: u32 = 200;

/// One pipe of a line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pipe {
    pub length_m: f64,
    pub sections: usize,
    pub diameter_m: f64,
    pub roughness_m: f64,
    /// Wave speed of the liquid in this pipe with no free gas: the liquid's
    /// compressibility and the wall's stretch.
    pub wave_speed_m_s: f64,
    /// The wall's share of the compliance `1/(rho a^2)`, from 0 to 1.
    pub wall_share: f64,
}

/// The liquid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Liquid {
    pub density_kg_m3: f64,
    pub kinematic_viscosity_m2_s: f64,
    pub vapour_pressure_pa: f64,
}

/// One run: the line, the supply, the valve and the time.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub pipes: Vec<Pipe>,
    pub liquid: Liquid,
    /// Absolute pressure of the supply at the line's inlet, held.
    pub supply_pressure_pa: f64,
    /// Absolute pressure downstream of the valve, held.
    pub return_pressure_pa: f64,
    /// Steady volume flow before the valve moves, toward the valve.
    pub initial_flow_m3_s: f64,
    /// The valve's open area falls linearly to zero over this time; zero is
    /// a closure within the first step.
    pub closure_time_s: f64,
    pub duration_s: f64,
    /// Volume of free gas per volume of liquid at [`ATMOSPHERE_PA`].
    pub free_gas_fraction_at_atmosphere: f64,
    /// Absolute pressure at which the wall first yields. Past it the run
    /// stops by name.
    pub wall_yield_pressure_pa: f64,
    /// The largest Mach number the line equations stand for: the flow
    /// speed over the local wave speed.
    pub mach_limit: f64,
    /// The largest number of samples of the histories.
    pub samples: usize,
    /// A wrong step on purpose, for the tests that attack the ledgers and
    /// the second law. Every served run passes [`Tamper::None`].
    pub tamper: Tamper,
}

/// What a run may carry beyond a line of one wave speed with its free gas:
/// the liquid's own stiffening under pressure, a valve that closes as a
/// measured stroke, the vapour's room as a share of the line, and the nodes
/// whose pressure is kept at every step. [`run`] is [`run_with`] with none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extensions {
    pub stiffening: Option<Stiffening>,
    pub valve_schedule: Option<ValveSchedule>,
    /// With a share, the vapour cavities of all the nodes together may take
    /// this share of the line's volume, and past it the run stops as
    /// [`LineStop::CavityLargerThanReach`] with the cavities over that room;
    /// without, each node's cavity may take its own reach (at the valve one
    /// section).
    pub vapour_share_limit: Option<f64>,
    /// Nodes whose absolute pressure is kept at every step.
    pub probe_nodes: Vec<usize>,
    /// Churchill's factor read from a table made for the run, linear in
    /// `ln Re` every 1/256 of a natural logarithm from `Re` = 1 to 1e9
    /// (within about 2e-6 of the formula), and the formula itself outside;
    /// the factor taken once a step at each section's velocity from the step
    /// before, its friction then settled to 1e-9 at that factor.
    pub friction_table: bool,
    /// With a limit, the free gas's share of the volume at every node with
    /// gas, at its pressure ([`void_fraction`]), is held to it at every step,
    /// and past it the run stops as [`LineStop::VoidAboveBubblyFlow`].
    pub void_fraction_limit: Option<f64>,
}

/// Churchill's factor of one pipe as a table in `ln Re`.
#[derive(Clone, Debug)]
struct FrictionTable {
    f: Vec<f64>,
}

impl FrictionTable {
    const PER_UNIT: f64 = 256.0;
    const TOP_LN_RE: f64 = 20.723_265_836_946_41; // ln 1e9

    fn new(relative_roughness: f64) -> FrictionTable {
        let n = (Self::TOP_LN_RE * Self::PER_UNIT).ceil() as usize + 2;
        let f = (0..n).map(|k| churchill_friction_factor(math::exp(k as f64 / Self::PER_UNIT), relative_roughness)).collect();
        FrictionTable { f }
    }

    fn factor(&self, reynolds: f64, relative_roughness: f64) -> f64 {
        let x = math::log(reynolds) * Self::PER_UNIT;
        if !(x >= 0.0 && x < (self.f.len() - 1) as f64) {
            return churchill_friction_factor(reynolds, relative_roughness);
        }
        let k = x as usize;
        let w = x - k as f64;
        self.f[k] + w * (self.f[k + 1] - self.f[k])
    }
}

/// Friction force per volume through a run's table.
fn friction_per_volume_table(velocity_m_s: f64, pipe: &Pipe, liquid: &Liquid, table: &FrictionTable) -> f64 {
    let speed = velocity_m_s.abs();
    if speed == 0.0 {
        return 0.0;
    }
    let reynolds = speed * pipe.diameter_m / liquid.kinematic_viscosity_m2_s;
    if laminar_limit(reynolds) {
        return laminar_friction_per_volume(velocity_m_s, pipe, liquid);
    }
    let f = table.factor(reynolds, pipe.roughness_m / pipe.diameter_m);
    liquid.density_kg_m3 * f * velocity_m_s * speed / (2.0 * pipe.diameter_m)
}

/// The liquid's compressibility as a function of pressure. The sections
/// carry the liquid and the wall at the stiffest, the top pressure; at
/// lower pressures the liquid is more compliant, and the difference is
/// lumped at the nodes, each holding it for the liquid of its reach, as the
/// free gas is. A node's lump then has the volume
///
/// ```text
/// b(p) = R S(p),     S(p) = integral from p to p_top of e(p') dp'
/// ```
///
/// with `R` the node's reach and `e` the excess compliance per volume, and
/// the stored energy `R` times the integral from the supply pressure to `p`
/// of `(p' - p_s) e(p')`. `e` is linear between the given pressures, so both
/// are closed forms, and `b` falls strictly with `p` while `e > 0`: the
/// node's energy is convex in its volume and the step that takes the
/// pressure at its end never gives energy back. Past the vapour pressure,
/// the bottom of the law, the node holds a vapour cavity beside its lump, as
/// a node with no lump does. A run with a stiffening carries no free gas.
#[derive(Clone, Debug, PartialEq)]
pub struct Stiffening {
    /// Absolute pressures, rising, from the liquid's vapour pressure to the
    /// top, the pressure the sections' wave speed is taken at.
    pub pressure_pa: Vec<f64>,
    /// The liquid's compliance per volume beyond the sections', 1/Pa, at each
    /// pressure: falling, and zero at the top.
    pub excess_compliance_per_pa: Vec<f64>,
}

/// A valve that closes as a measured stroke: its open fraction at equal
/// steps of time from the start, linear between them, and the last value
/// after them.
#[derive(Clone, Debug, PartialEq)]
pub struct ValveSchedule {
    pub step_s: f64,
    pub open_fraction: Vec<f64>,
}

impl ValveSchedule {
    fn at(&self, time_s: f64) -> f64 {
        let x = time_s / self.step_s;
        let last = self.open_fraction.len() - 1;
        if !(x < last as f64) {
            return self.open_fraction[last];
        }
        let k = x.floor() as usize;
        let w = x - k as f64;
        self.open_fraction[k] + w * (self.open_fraction[k + 1] - self.open_fraction[k])
    }
}

/// The lumped law of a run: the stiffening's pressures and excess
/// compliances, the integral `S` at each pressure from it to the top, and
/// `W`, the integral of `(p - p_s) e` from the bottom to each pressure.
#[derive(Clone, Debug)]
struct LumpLaw {
    p: Vec<f64>,
    e: Vec<f64>,
    s: Vec<f64>,
    w: Vec<f64>,
    /// Each segment's slope of `e` in the pressure.
    slope: Vec<f64>,
    supply: f64,
    w_supply: f64,
}

impl LumpLaw {
    fn new(stiff: &Stiffening, supply: f64) -> LumpLaw {
        let (p, e) = (stiff.pressure_pa.clone(), stiff.excess_compliance_per_pa.clone());
        let n = p.len();
        let mut s = vec![0.0f64; n];
        for k in (0..n - 1).rev() {
            s[k] = s[k + 1] + 0.5 * (e[k] + e[k + 1]) * (p[k + 1] - p[k]);
        }
        let mut w = vec![0.0f64; n];
        for k in 0..n - 1 {
            w[k + 1] = w[k] + Self::segment_w(p[k], e[k], (e[k + 1] - e[k]) / (p[k + 1] - p[k]), p[k + 1] - p[k], supply);
        }
        let slope = (0..n - 1).map(|k| (e[k + 1] - e[k]) / (p[k + 1] - p[k])).collect();
        let mut law = LumpLaw { p, e, s, w, slope, supply, w_supply: 0.0 };
        law.w_supply = law.w_at(supply);
        law
    }

    /// The integral of `(p - p_s) e` over `u` from a segment's start.
    fn segment_w(p_k: f64, e_k: f64, slope: f64, u: f64, supply: f64) -> f64 {
        (p_k - supply) * (e_k * u + 0.5 * slope * u * u) + 0.5 * e_k * u * u + slope * u * u * u / 3.0
    }

    fn top(&self) -> f64 {
        self.p[self.p.len() - 1]
    }

    fn bottom(&self) -> f64 {
        self.p[0]
    }

    fn segment(&self, pressure: f64) -> usize {
        self.p.partition_point(|x| *x <= pressure).clamp(1, self.p.len() - 1) - 1
    }

    /// `S(p)`, the lump's volume per reach, for a pressure from the bottom up.
    fn s_at(&self, pressure: f64) -> f64 {
        if pressure >= self.top() {
            return 0.0;
        }
        let k = self.segment(pressure);
        let (h, u) = (self.p[k + 1] - self.p[k], pressure - self.p[k]);
        let slope = (self.e[k + 1] - self.e[k]) / h;
        // From p to the segment's end, then the rest to the top.
        self.s[k + 1] + self.e[k] * (h - u) + 0.5 * slope * (h * h - u * u)
    }

    fn w_at(&self, pressure: f64) -> f64 {
        let pressure = pressure.min(self.top());
        let k = self.segment(pressure);
        let slope = (self.e[k + 1] - self.e[k]) / (self.p[k + 1] - self.p[k]);
        self.w[k] + Self::segment_w(self.p[k], self.e[k], slope, pressure - self.p[k], self.supply)
    }

    /// The segment where `s[k+1] <= s <= s[k]`, walking from a hint: a
    /// node's volume moves little in a step.
    fn find_s(&self, s: f64, hint: usize) -> usize {
        let last = self.s.len() - 2;
        let mut k = hint.min(last);
        let mut walked = 0;
        while k > 0 && s > self.s[k] {
            k -= 1;
            walked += 1;
            if walked > 8 {
                return self.s.partition_point(|x| *x > s).clamp(1, self.s.len() - 1) - 1;
            }
        }
        while k < last && s < self.s[k + 1] {
            k += 1;
            walked += 1;
            if walked > 8 {
                return self.s.partition_point(|x| *x > s).clamp(1, self.s.len() - 1) - 1;
            }
        }
        k
    }

    /// The pressure, the excess compliance there and the segment, at a
    /// volume per reach `s` from zero (the top) to `S` at the bottom.
    fn locate(&self, s: f64, hint: usize) -> (f64, f64, usize) {
        if s <= 0.0 {
            let k = self.p.len() - 2;
            return (self.top(), 0.0, k);
        }
        if s >= self.s[0] {
            return (self.bottom(), self.e[0], 0);
        }
        let k = self.find_s(s, hint);
        let h = self.p[k + 1] - self.p[k];
        let slope = self.slope[k];
        let c = (self.s[k] - s).max(0.0);
        let root = (self.e[k] * self.e[k] + 2.0 * slope * c).max(0.0).sqrt();
        let u = (2.0 * c / (self.e[k] + root)).min(h);
        (self.p[k] + u, self.e[k] + slope * u, k)
    }

    /// `W` at a pressure inside segment `k`.
    fn w_in(&self, pressure: f64, k: usize) -> f64 {
        self.w[k] + Self::segment_w(self.p[k], self.e[k], self.slope[k], pressure - self.p[k], self.supply)
    }

    /// The pressure at which the lump's volume per reach is `s`, from zero
    /// (the top) to `S` at the bottom.
    fn pressure_at(&self, s: f64) -> f64 {
        if s <= 0.0 {
            return self.top();
        }
        if s >= self.s[0] {
            return self.bottom();
        }
        // S falls with the pressure: the segment where s[k+1] <= s <= s[k].
        let k = self.s.partition_point(|x| *x > s).clamp(1, self.s.len() - 1) - 1;
        let h = self.p[k + 1] - self.p[k];
        let slope = (self.e[k + 1] - self.e[k]) / h;
        // The integral from p_k to p, c, is s[k] - s; solve e_k u + slope u^2 / 2 = c.
        let c = (self.s[k] - s).max(0.0);
        let root = (self.e[k] * self.e[k] + 2.0 * slope * c).max(0.0).sqrt();
        let u = 2.0 * c / (self.e[k] + root);
        (self.p[k] + u).min(self.p[k + 1])
    }
}

/// A node's own law: the gas cavity of line_transient, or the lumped
/// stiffening of reach `reach` with its vapour cavity.
#[derive(Clone, Copy, Debug)]
struct Lump<'a> {
    law: &'a LumpLaw,
    reach: f64,
    vapour_pa: f64,
}

impl Lump<'_> {
    /// The lump's volume at the vapour pressure: past it the node holds vapour.
    fn vapour_volume(&self) -> f64 {
        self.reach * self.law.s[0]
    }

    fn pressure(&self, b: f64) -> f64 {
        if b >= self.vapour_volume() { self.vapour_pa } else { self.law.pressure_at(b / self.reach) }
    }

    /// The lump's stored energy and the vapour cavity's, from the node at
    /// the supply pressure.
    fn energies(&self, b: f64) -> (f64, f64) {
        let b_v = self.vapour_volume();
        if b <= b_v {
            (self.reach * (self.law.w_at(self.pressure(b)) - self.law.w_supply), 0.0)
        } else {
            (self.reach * (self.law.w[0] - self.law.w_supply), (self.law.supply - self.vapour_pa) * (b - b_v))
        }
    }

    fn energy(&self, b: f64) -> f64 {
        let (lump, vapour) = self.energies(b);
        lump + vapour
    }

    /// A bound on the size of the terms the lump's and the vapour's energies,
    /// `lumped` and `vapour` at volume `b`, are made of.
    fn magnitude(&self, b: f64, lumped: f64, vapour: f64) -> f64 {
        let b_v = self.vapour_volume();
        let lump = lumped.abs() + 2.0 * self.reach * self.law.w_supply.abs();
        if b <= b_v { lump } else { lump + vapour.abs() + (self.law.supply - self.vapour_pa) * (b + b_v) }
    }

    /// The pressure, the excess compliance and the segment at a volume.
    fn state(&self, b: f64, hint: usize) -> (f64, f64, usize) {
        if b >= self.vapour_volume() { (self.vapour_pa, self.law.e[0], 0) } else { self.law.locate(b / self.reach, hint) }
    }

    /// [`Lump::energies`] from a located state.
    fn energies_at(&self, b: f64, pressure: f64, k: usize) -> (f64, f64) {
        let b_v = self.vapour_volume();
        if b <= b_v {
            (self.reach * (self.law.w_in(pressure, k) - self.law.w_supply), 0.0)
        } else {
            (self.reach * (self.law.w[0] - self.law.w_supply), (self.law.supply - self.vapour_pa) * (b - b_v))
        }
    }
}

/// The volume a node is left with when the flows at a gauge pressure run for
/// a step: the old less what came in.
fn old_volume_less_inflow(kind: &NodeKind, old: f64, gauge: f64, supply: f64, dt: f64) -> f64 {
    old - kind.net_in(gauge, supply) * dt
}

/// A lumped node's step: its new volume and gauge pressure, or, where even
/// a lump squeezed to nothing cannot take the inflow, word that the node is
/// driven to the top of its law, where its lump is empty.
enum LumpStep {
    /// With the pressure, excess compliance and segment of the new volume,
    /// and the step's [`NodeStep::root_gap_j`].
    Settled { volume: f64, gauge: f64, passes: u32, located: (f64, f64, usize), root_gap_j: f64, carried_gap_j: f64 },
    AboveTop,
    NotSettled,
}

fn lump_step(kind: &NodeKind, lump: &Lump, old: f64, dt: f64, hint: &mut usize) -> LumpStep {
    if let NodeKind::Inner { cp_in, z_left, cm_in, z_right } = *kind {
        if let Some(step) = lump_step_inner(cp_in / z_left + cm_in / z_right, 1.0 / z_left + 1.0 / z_right, kind, lump, old, dt, hint) {
            return step;
        }
    }
    lump_step_bracketed(kind, lump, old, dt)
}

/// Between two sections the flows are linear in the pressure, `net_in = s
/// - k p_g`, so `G(b) = b - old + dt net_in(p(b))` is concave and rising in
/// the volume (`p(b)` is convex and falling): Newton's method from the old
/// volume lands at or left of the root after its first step and then rises
/// to it. None hands the step to the bracketed search.
#[allow(clippy::too_many_arguments)]
fn lump_step_inner(s: f64, k: f64, kind: &NodeKind, lump: &Lump, old: f64, dt: f64, hint: &mut usize) -> Option<LumpStep> {
    let supply = lump.law.supply;
    let b_v = lump.vapour_volume();
    let mut b = old;
    let mut passes = 0u32;
    // The state at b; the loop ends where Newton's next move would be under
    // a part in 1e12 of the volume, and takes that b.
    let located = loop {
        passes += 1;
        if passes > 60 {
            return None;
        }
        let located = lump.state(b, *hint);
        *hint = located.2;
        let gb = b - old + dt * (s - k * (located.0 - supply));
        let d = if b >= b_v {
            1.0
        } else if located.1 > 0.0 {
            1.0 + dt * k / (lump.reach * located.1)
        } else {
            f64::INFINITY
        };
        if !d.is_finite() {
            return None;
        }
        let move_by = gb / d;
        if move_by.abs() <= 1.0e-12 * b {
            break located;
        }
        let next = b - move_by;
        if !(next > 0.0) {
            return None;
        }
        b = next;
    };
    let gauge = located.0 - supply;
    // The volume the flows at that pressure leave, so the volume ledger
    // closes to rounding; it is b to a part in 1e12.
    let volume = old - kind.net_in(gauge, supply) * dt;
    if !(volume >= 0.0) {
        return None;
    }
    // The energy is taken at b, the work at the volume the flows leave: the
    // two differ by at most the larger of the pressures at b and at the
    // volume, times their distance; the pressure at the volume is b's less
    // that distance over the lump's compliance there. The node carries the
    // same into its next step.
    let off = (b - volume).abs();
    let root_gap_j = off * (gauge.abs() + off / (lump.reach * located.1));
    Some(LumpStep::Settled { volume, gauge, passes, located, root_gap_j, carried_gap_j: root_gap_j })
}

fn lump_step_bracketed(kind: &NodeKind, lump: &Lump, old: f64, dt: f64) -> LumpStep {
    let supply = lump.law.supply;
    let g = |b: f64| b - old + dt * kind.net_in(lump.pressure(b) - supply, supply);
    let at_zero = g(0.0);
    if at_zero >= 0.0 {
        return LumpStep::AboveTop;
    }
    let (mut lo, mut g_lo) = (0.0f64, at_zero);
    let mut hi = old.max(lump.vapour_volume() * 1.0e-6).max(f64::MIN_POSITIVE);
    let mut g_hi = g(hi);
    let mut passes = 1u32;
    while g_hi <= 0.0 {
        lo = hi;
        g_lo = g_hi;
        hi *= 2.0;
        g_hi = g(hi);
        passes += 1;
        if passes > MAX_NODE_PASSES || !hi.is_finite() {
            return LumpStep::NotSettled;
        }
    }
    // False position, the Illinois way, with a bisection whenever an end has
    // stayed put twice; past its passes, halving to the bracket's last bit.
    let mut side = 0i32;
    loop {
        passes += 1;
        if passes > MAX_NODE_PASSES + MAX_BISECTION_PASSES {
            return LumpStep::NotSettled;
        }
        let mut b = if passes > MAX_NODE_PASSES { 0.5 * (lo + hi) } else { hi - g_hi * (hi - lo) / (g_hi - g_lo) };
        if !(b > lo && b < hi) {
            b = 0.5 * (lo + hi);
        }
        if !(b > lo && b < hi) {
            break;
        }
        let gb = g(b);
        if gb == 0.0 {
            lo = b;
            hi = b;
            break;
        }
        if gb < 0.0 {
            lo = b;
            g_lo = gb;
            if side == -1 {
                g_hi *= 0.5;
            }
            side = -1;
        } else {
            hi = b;
            g_hi = gb;
            if side == 1 {
                g_lo *= 0.5;
            }
            side = 1;
        }
        if hi - lo <= 4.0 * f64::EPSILON * hi {
            break;
        }
    }
    let b = 0.5 * (lo + hi);
    let gauge = lump.pressure(b) - supply;
    // The volume the flows at that pressure leave, so the volume ledger
    // closes to rounding.
    let volume = old - kind.net_in(gauge, supply) * dt;
    if !(volume >= 0.0) {
        return LumpStep::AboveTop;
    }
    let located = lump.state(volume, 0);
    let root_gap_j = root_gap(gauge, located.0 - supply, old, volume);
    LumpStep::Settled { volume, gauge, passes, located, root_gap_j, carried_gap_j: 0.0 }
}

/// A wrong step a test may ask for, to show it is stopped by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tamper {
    /// The step as it is.
    None,
    /// The valve's work is left out of the energy ledger.
    ValveWorkDropped,
    /// Every wave leaving a node between two sections is a millionth larger.
    WaveGain,
    /// Friction pushes the flow instead of holding it back.
    FrictionPushes,
    /// A node with gas takes the pressure of its cavity at the step's start,
    /// a forward step, in place of the end's.
    CavityForward,
    /// A node with a lumped stiffening takes its pressure at the step's
    /// start, a forward step, in place of the end's.
    LumpForward,
    /// The lumped stiffening's energy is left out of the energy ledger.
    LumpEnergyDropped,
}

/// Why a run stopped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineStop {
    /// The supply cannot drive the asked flow: the steady pressure at the
    /// valve is not above the return pressure.
    SupplyCannotDriveFlow { valve_pressure_pa: f64 },
    /// The steady pressure somewhere on the line is not above the vapour
    /// pressure.
    SteadyBelowVapour { pressure_pa: f64 },
    /// The pressure at a node rose to where the wall first yields.
    WallYield { time_s: f64, position_m: f64, pressure_pa: f64 },
    /// The flow speed at a node reached the Mach limit of the line
    /// equations, against the local wave speed of liquid and free gas.
    LineMachAboveAcoustic { time_s: f64, position_m: f64, mach: f64 },
    /// A node's cavity grew past the liquid it is lumped from: between two
    /// sections, larger than its reach; at the valve, longer than one
    /// section, so that the column's end has passed the next node.
    CavityLargerThanReach { time_s: f64, position_m: f64, cavity_over_reach: f64 },
    /// The free gas at a node expanded past [`Extensions::void_fraction_limit`]
    /// of the volume.
    VoidAboveBubblyFlow { time_s: f64, position_m: f64, void_fraction: f64 },
    /// The friction of a section did not settle.
    FrictionNotSettled { time_s: f64, position_m: f64 },
    /// A node's root search did not settle.
    NodeNotSettled { time_s: f64, position_m: f64 },
    /// A value was NaN or infinite.
    NonFiniteValue { time_s: f64 },
    /// The energy ledger did not close.
    EnergyLedgerBroken { time_s: f64, residual: f64 },
    /// The volume ledger did not close.
    VolumeLedgerBroken { time_s: f64, residual: f64 },
    /// A section's friction gave energy to the flow.
    FrictionGaveEnergy { time_s: f64, position_m: f64, work_j: f64 },
    /// A node's cavity update gave energy to the waves.
    CavityGaveEnergy { time_s: f64, position_m: f64, work_j: f64 },
}

/// The ledger at one time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ledger {
    pub kinetic_j: f64,
    pub liquid_compression_j: f64,
    pub wall_strain_j: f64,
    pub cavity_j: f64,
    /// The liquid's own compression beyond the sections', lumped at the
    /// nodes ([`Stiffening`]); zero without one.
    pub liquid_lump_j: f64,
    pub friction_loss_j: f64,
    pub valve_work_j: f64,
    pub cavity_loss_j: f64,
    pub stored_volume_m3: f64,
    pub cavity_volume_m3: f64,
    pub supply_volume_m3: f64,
    pub valve_volume_m3: f64,
}

impl Ledger {
    /// Energy stored in the line and its cavities.
    pub fn stored_j(&self) -> f64 {
        self.kinetic_j + self.liquid_compression_j + self.wall_strain_j + self.cavity_j + self.liquid_lump_j
    }
    /// Energy that has left the line: friction, the valve and collapses.
    pub fn gone_j(&self) -> f64 {
        self.friction_loss_j + self.valve_work_j + self.cavity_loss_j
    }
}

/// What a run returns.
#[derive(Clone, Debug, PartialEq)]
pub struct LineRun {
    pub time_step_s: f64,
    pub steps: u64,
    /// The wave speed each pipe was run at, with its section length over the
    /// time step.
    pub run_wave_speeds_m_s: Vec<f64>,
    pub node_positions_m: Vec<f64>,
    /// Steady absolute pressure at each node before the valve moves.
    pub steady_pressures_pa: Vec<f64>,
    /// The valve's flow coefficient, `Q = c tau sqrt(2 dp / rho)`, from the
    /// steady flow.
    pub valve_coefficient_m2: f64,
    pub sample_times_s: Vec<f64>,
    pub valve_pressure_pa: Vec<f64>,
    pub valve_flow_m3_s: Vec<f64>,
    pub midpoint_pressure_pa: Vec<f64>,
    pub valve_cavity_m3: Vec<f64>,
    /// The ledger at each sample.
    pub ledger_samples: Vec<Ledger>,
    /// The ledger at the start and at the end.
    pub ledger_start: Ledger,
    pub ledger_end: Ledger,
    /// Every step's valve pressure, for the field.
    pub valve_pressure_every_step_pa: Vec<f64>,
    pub pressure_max_along_line_pa: Vec<f64>,
    pub pressure_min_along_line_pa: Vec<f64>,
    pub valve_pressure_max_pa: f64,
    pub valve_pressure_max_time_s: f64,
    pub valve_pressure_min_pa: f64,
    pub line_pressure_max_pa: f64,
    pub line_pressure_min_pa: f64,
    pub cavity_volume_max_m3: f64,
    pub largest_cavity_over_reach: f64,
    pub largest_mach: f64,
    /// Time of the first vapour cavity at each node that had one.
    pub first_vapour_time_s: Vec<f64>,
    pub first_vapour_position_m: Vec<f64>,
    pub final_pressures_pa: Vec<f64>,
    pub final_velocities_m_s: Vec<f64>,
    pub energy_scale_j: f64,
    pub volume_scale_m3: f64,
    pub largest_energy_residual: f64,
    pub largest_volume_residual: f64,
    pub friction_passes_max: u32,
    pub node_passes_max: u32,
    /// The absolute pressure at each of [`Extensions::probe_nodes`], at the
    /// start and after every step.
    pub probe_pressures_pa: Vec<Vec<f64>>,
    /// The vapour at the valve at each sample, beside the lump there.
    pub valve_vapour_m3: Vec<f64>,
    /// The most vapour in the whole line at once.
    pub vapour_volume_max_m3: f64,
}

/// Churchill's (1977) Darcy friction factor, one formula over laminar,
/// transitional and turbulent flow. Below a Reynolds number of about
/// 1.6e-25 the formula's laminar term `(8/Re)^12` passes the largest 64-bit
/// number; there the turbulent terms are under 1e-300 of it and the formula
/// is its laminar limit, `64/Re`, which is returned.
pub fn churchill_friction_factor(reynolds: f64, relative_roughness: f64) -> f64 {
    if reynolds <= 0.0 {
        return 0.0;
    }
    if laminar_limit(reynolds) {
        return 64.0 / reynolds;
    }
    let inner = math::exp(0.9 * math::log(7.0 / reynolds)) + 0.27 * relative_roughness;
    let a = math::powi(2.457 * math::log(1.0 / inner), 16);
    let b = math::powi(37_530.0 / reynolds, 16);
    let laminar = math::powi(8.0 / reynolds, 12);
    let sum = a + b;
    8.0 * math::exp(math::log(laminar + 1.0 / (sum * sum.sqrt())) / 12.0)
}

/// Whether Churchill's laminar term `(8/Re)^12` passes the largest 64-bit
/// number: the flow is then laminar to every bit of the factor.
fn laminar_limit(reynolds: f64) -> bool {
    !math::powi(8.0 / reynolds, 12).is_finite()
}

/// The laminar friction force per volume, `32 mu V / D^2` (Poiseuille's,
/// `64/Re` in Darcy's form), for a flow in [`laminar_limit`]: written so
/// that no factor passes the 64-bit range however slow the flow.
fn laminar_friction_per_volume(velocity_m_s: f64, pipe: &Pipe, liquid: &Liquid) -> f64 {
    32.0 * liquid.density_kg_m3 * liquid.kinematic_viscosity_m2_s * velocity_m_s / (pipe.diameter_m * pipe.diameter_m)
}

/// Friction force per volume of a velocity in a pipe, signed with it.
pub fn friction_per_volume(velocity_m_s: f64, pipe: &Pipe, liquid: &Liquid) -> f64 {
    let speed = velocity_m_s.abs();
    if speed == 0.0 {
        return 0.0;
    }
    let reynolds = speed * pipe.diameter_m / liquid.kinematic_viscosity_m2_s;
    if laminar_limit(reynolds) {
        return laminar_friction_per_volume(velocity_m_s, pipe, liquid);
    }
    let f = churchill_friction_factor(reynolds, pipe.roughness_m / pipe.diameter_m);
    liquid.density_kg_m3 * f * velocity_m_s * speed / (2.0 * pipe.diameter_m)
}

fn area(pipe: &Pipe) -> f64 {
    0.25 * std::f64::consts::PI * pipe.diameter_m * pipe.diameter_m
}

/// A node's cavity: its gas constant `M = p V_gas` and the pressures that
/// bound its stored energy.
#[derive(Clone, Copy, Debug)]
struct Cavity {
    gas_pa_m3: f64,
    supply_pa: f64,
    vapour_pa: f64,
}

impl Cavity {
    /// Volume at which the gas alone reaches the vapour pressure.
    fn vapour_volume(&self) -> f64 {
        self.gas_pa_m3 / self.vapour_pa
    }

    /// Stored energy of a cavity of whole volume `v`, from the cavity at the
    /// supply pressure.
    fn energy(&self, v: f64) -> f64 {
        let m = self.gas_pa_m3;
        if m > 0.0 {
            let v_star = self.vapour_volume();
            let reference = m / self.supply_pa;
            let gas = |volume: f64| m * math::log(reference / volume) + self.supply_pa * (volume - reference);
            if v <= v_star { gas(v) } else { gas(v_star) + (self.supply_pa - self.vapour_pa) * (v - v_star) }
        } else {
            (self.supply_pa - self.vapour_pa) * v
        }
    }

    /// The pressure of a cavity of volume `v` at rest.
    fn pressure(&self, v: f64) -> f64 {
        if self.gas_pa_m3 > 0.0 && v < self.vapour_volume() { self.gas_pa_m3 / v } else { self.vapour_pa }
    }

    /// A bound on the size of the terms [`Cavity::energy`] adds to make
    /// `energy`, its value at `v`: what the rounding of that sum is a part of.
    fn magnitude(&self, v: f64, energy: f64) -> f64 {
        let m = self.gas_pa_m3;
        if m > 0.0 {
            let v_star = self.vapour_volume();
            let reference = m / self.supply_pa;
            // |M ln(V_ref/V)| is at most |U| + p_s |V - V_ref|.
            let gas = energy.abs() + 2.0 * self.supply_pa * (v.min(v_star) + reference);
            if v <= v_star { gas } else { gas + (self.supply_pa - self.vapour_pa) * (v + v_star) }
        } else {
            energy.abs()
        }
    }
}

/// What flows at a node, as functions of the node's absolute pressure.
#[derive(Clone, Copy, Debug)]
enum NodeKind {
    /// Between two sections: incoming `C+` from the left, `C-` from the right.
    Inner { cp_in: f64, z_left: f64, cm_in: f64, z_right: f64 },
    /// At the valve: incoming `C+` from the left, and the valve's flow.
    Valve { cp_in: f64, z_left: f64, opening_m2: f64, return_pa: f64, density: f64 },
}

impl NodeKind {
    /// Net volume flow into the node at gauge pressure `pg` (over the supply).
    fn net_in(&self, pg: f64, supply: f64) -> f64 {
        match *self {
            NodeKind::Inner { cp_in, z_left, cm_in, z_right } => (cp_in - pg) / z_left - (pg - cm_in) / z_right,
            NodeKind::Valve { cp_in, z_left, opening_m2, return_pa, density } => {
                (cp_in - pg) / z_left - valve_flow(opening_m2, pg + supply - return_pa, density)
            }
        }
    }

    /// The gauge pressure at which `net_in` equals `target` (a volume flow):
    /// it falls strictly with the pressure, so the root is unique.
    fn pressure_for_net_in(&self, target: f64, supply: f64) -> f64 {
        match *self {
            NodeKind::Inner { cp_in, z_left, cm_in, z_right } => {
                (cp_in / z_left + cm_in / z_right - target) / (1.0 / z_left + 1.0 / z_right)
            }
            NodeKind::Valve { cp_in, z_left, opening_m2, return_pa, density } => {
                // (cp - pg)/Z - target = c sgn(x) sqrt(2|x|/rho), x = pg + supply - return.
                let shifted = cp_in - target * z_left;
                let zero_flow = shifted + supply - return_pa;
                if opening_m2 == 0.0 {
                    return shifted;
                }
                let beta = opening_m2 * (2.0 / density).sqrt() * z_left;
                let x = if zero_flow >= 0.0 {
                    let s = 2.0 * zero_flow / (beta + (beta * beta + 4.0 * zero_flow).sqrt());
                    s * s
                } else {
                    let s = -2.0 * zero_flow / (beta + (beta * beta - 4.0 * zero_flow).sqrt());
                    -(s * s)
                };
                x + return_pa - supply
            }
        }
    }
}

/// Flow through the valve of open area times coefficient `opening_m2` under a
/// pressure difference `dp`, signed with it.
fn valve_flow(opening_m2: f64, dp: f64, density: f64) -> f64 {
    if dp >= 0.0 { opening_m2 * (2.0 * dp / density).sqrt() } else { -opening_m2 * (-2.0 * dp / density).sqrt() }
}

/// One node's step: its new cavity volume, its gauge pressure over the step
/// and the energy lost to a collapse.
struct NodeStep {
    volume: f64,
    gauge: f64,
    collapse_loss: f64,
    passes: u32,
    /// What the step's own root leaves unsettled in its second law: the
    /// pressure the flows were taken at against the pressure of the volume
    /// they leave, times the volume moved (or, where the energy is taken at
    /// the root, the pressure times the root's distance from that volume).
    /// The node's update may give the waves no more than this, with the
    /// rounding of its own terms. Zero for a step that is not the backward
    /// step's root.
    root_gap_j: f64,
}

/// The pressure, the gauge and the volume moved of a step whose flows were
/// taken at `gauge` and leave `volume`: the second law's slack of the root.
fn root_gap(gauge: f64, gauge_of_volume: f64, old: f64, volume: f64) -> f64 {
    ((gauge - gauge_of_volume) * (old - volume)).abs()
}

/// Passes of halving after false position has spent its own: a bracket of
/// 64-bit numbers is down to two neighbours in at most this many.
pub const MAX_BISECTION_PASSES: u32 = 2_200;

fn node_step(kind: &NodeKind, cavity: &Cavity, old: f64, dt: f64, forward: bool) -> Option<NodeStep> {
    let supply = cavity.supply_pa;
    let vapour_gauge = cavity.vapour_pa - supply;
    if cavity.gas_pa_m3 == 0.0 {
        if old == 0.0 {
            let gauge = kind.pressure_for_net_in(0.0, supply);
            if gauge >= vapour_gauge {
                return Some(NodeStep { volume: 0.0, gauge, collapse_loss: 0.0, passes: 0, root_gap_j: 0.0 });
            }
            let volume = -kind.net_in(vapour_gauge, supply) * dt;
            return Some(NodeStep { volume: volume.max(0.0), gauge: vapour_gauge, collapse_loss: 0.0, passes: 0, root_gap_j: 0.0 });
        }
        let volume = old - kind.net_in(vapour_gauge, supply) * dt;
        if volume > 0.0 {
            return Some(NodeStep { volume, gauge: vapour_gauge, collapse_loss: 0.0, passes: 0, root_gap_j: 0.0 });
        }
        // The cavity closes within the step: the liquid fills what is left.
        let gauge = kind.pressure_for_net_in(old / dt, supply);
        let gauge = gauge.max(vapour_gauge);
        let collapse_loss = (gauge - vapour_gauge) * old;
        return Some(NodeStep { volume: 0.0, gauge, collapse_loss, passes: 0, root_gap_j: 0.0 });
    }
    if forward {
        let gauge = cavity.pressure(old) - supply;
        let volume = old - kind.net_in(gauge, supply) * dt;
        return if volume > 0.0 { Some(NodeStep { volume, gauge, collapse_loss: 0.0, passes: 1, root_gap_j: 0.0 }) } else { None };
    }
    // The volume the flows at the root's pressure leave, so the volume
    // ledger closes to rounding; where the gas is so small that the rounding
    // of the flows takes that volume to zero or below, the root itself,
    // which it is to that rounding.
    let settle = |b: f64, gauge: f64, passes: u32| -> Option<NodeStep> {
        let from_flows = old - kind.net_in(gauge, supply) * dt;
        let volume = if from_flows > 0.0 { from_flows } else { b };
        if !(volume > 0.0) {
            return None;
        }
        let root_gap_j = root_gap(gauge, cavity.pressure(volume) - supply, old, volume);
        Some(NodeStep { volume, gauge, collapse_loss: 0.0, passes, root_gap_j })
    };
    // Gas between two sections: the flows are linear in the pressure,
    // net_in = s - k p_g, so with the gas law the new volume is the root of
    // a quadratic, or, past the vapour volume, of a straight line.
    if let NodeKind::Inner { cp_in, z_left, cm_in, z_right } = *kind {
        let k = 1.0 / z_left + 1.0 / z_right;
        let s = cp_in / z_left + cm_in / z_right;
        let beta = dt * (s + k * supply) - old;
        let gamma = dt * k * cavity.gas_pa_m3;
        let root = (beta * beta + 4.0 * gamma).sqrt();
        let b = if beta >= 0.0 { 2.0 * gamma / (beta + root) } else { 0.5 * (root - beta) };
        let gauge = if b <= cavity.vapour_volume() { cavity.gas_pa_m3 / b - supply } else { vapour_gauge };
        return settle(b, gauge, 1);
    }
    // Gas at the valve: G(b) = b - old + dt net_in(p(b)) rises strictly in
    // b; find its root in ln b.
    let g = |b: f64| b - old + dt * kind.net_in(cavity.pressure(b) - supply, supply);
    let at_old = g(old);
    if at_old == 0.0 {
        return Some(NodeStep { volume: old, gauge: cavity.pressure(old) - supply, collapse_loss: 0.0, passes: 0, root_gap_j: 0.0 });
    }
    let (mut lo, mut hi, mut g_lo, mut g_hi);
    let mut passes = 0u32;
    if at_old < 0.0 {
        lo = old;
        g_lo = at_old;
        hi = old;
        loop {
            hi *= 2.0;
            g_hi = g(hi);
            passes += 1;
            if g_hi > 0.0 {
                break;
            }
            lo = hi;
            g_lo = g_hi;
            if passes > MAX_NODE_PASSES || !hi.is_finite() {
                return None;
            }
        }
    } else {
        hi = old;
        g_hi = at_old;
        lo = old;
        loop {
            lo *= 0.5;
            g_lo = g(lo);
            passes += 1;
            if g_lo < 0.0 {
                break;
            }
            hi = lo;
            g_hi = g_lo;
            if passes > MAX_NODE_PASSES || lo == 0.0 {
                return None;
            }
        }
    }
    // False position on ln b, the Illinois way, with a bisection whenever an
    // end has stayed put twice; past its passes, halving in ln b to the
    // bracket's last bit.
    let (mut u_lo, mut u_hi) = (math::log(lo), math::log(hi));
    let mut side = 0i32;
    loop {
        passes += 1;
        if passes > MAX_NODE_PASSES + MAX_BISECTION_PASSES {
            return None;
        }
        let mut u = if passes > MAX_NODE_PASSES { 0.5 * (u_lo + u_hi) } else { u_hi - g_hi * (u_hi - u_lo) / (g_hi - g_lo) };
        if !(u > u_lo && u < u_hi) {
            u = 0.5 * (u_lo + u_hi);
        }
        let b = math::exp(u);
        if !(b > lo && b < hi) {
            break;
        }
        let gb = g(b);
        if gb == 0.0 {
            lo = b;
            hi = b;
            break;
        }
        if gb < 0.0 {
            lo = b;
            u_lo = u;
            g_lo = gb;
            if side == -1 {
                g_hi *= 0.5;
            }
            side = -1;
        } else {
            hi = b;
            u_hi = u;
            g_hi = gb;
            if side == 1 {
                g_lo *= 0.5;
            }
            side = 1;
        }
        if hi - lo <= 4.0 * f64::EPSILON * hi {
            break;
        }
    }
    let b = 0.5 * (lo + hi);
    let gauge = cavity.pressure(b) - supply;
    settle(b, gauge, passes)
}

/// One section's friction in flight: the pressure each wave loses, the
/// energy taken, and the passes.
#[allow(clippy::too_many_arguments)]
fn section_friction(cp: f64, cm: f64, z: f64, pipe: &Pipe, liquid: &Liquid, dx: f64, a: f64, guess: f64, table: Option<&FrictionTable>) -> Option<(f64, f64, f64, u32)> {
    let area = area(pipe);
    let rho_a = liquid.density_kg_m3 * a;
    let velocity_before = 0.5 * (cp - cm) / z / area;
    // The pass starts from the section's loss of the step before.
    let mut delta = guess;
    let mut passes = 0u32;
    // With a table the factor is read once, at the first pass's velocity; a
    // flow so slow that the factor passes the 64-bit range is laminar to
    // every bit, and its force is taken from Poiseuille's law at each pass.
    let factor = table
        .map(|t| {
            let v = velocity_before - 0.5 * delta / rho_a;
            t.factor(v.abs() * pipe.diameter_m / liquid.kinematic_viscosity_m2_s, pipe.roughness_m / pipe.diameter_m)
        })
        .filter(|f| f.is_finite());
    loop {
        passes += 1;
        let v_mid = velocity_before - 0.5 * delta / rho_a;
        let next = match factor {
            Some(f) => dx * liquid.density_kg_m3 * f * v_mid * v_mid.abs() / (2.0 * pipe.diameter_m),
            None => dx * friction_per_volume(v_mid, pipe, liquid),
        };
        // The friction factor's arithmetic carries a few units of the last
        // place, so the fixed point is settled to a part in 1e13; it
        // contracts by under 1e-3 a pass.
        if (next - delta).abs() <= if table.is_some() { 1e-9 } else { FRICTION_SETTLED } * next.abs() {
            // The force on the section, F dx A, times its velocity halfway
            // through this loss, times dt: what the two waves lose.
            let v_mid = velocity_before - 0.5 * next / rho_a;
            let work = next * area * v_mid * (dx / a);
            return Some((next, work, v_mid, passes));
        }
        if passes > MAX_FRICTION_PASSES || !next.is_finite() {
            return None;
        }
        delta = next;
    }
}

/// Runs the line.
pub fn run(spec: &Run) -> Result<LineRun, LineStop> {
    run_with(spec, &Extensions::default())
}

/// Runs the line with what [`Extensions`] carries.
pub fn run_with(spec: &Run, ext: &Extensions) -> Result<LineRun, LineStop> {
    let liquid = spec.liquid;
    let rho = liquid.density_kg_m3;
    let first = spec.pipes[0];
    let dt = first.length_m / first.sections as f64 / first.wave_speed_m_s;
    // Section tables.
    let mut sec_pipe: Vec<usize> = Vec::new();
    let mut run_speeds = Vec::new();
    for (index, pipe) in spec.pipes.iter().enumerate() {
        let dx = pipe.length_m / pipe.sections as f64;
        run_speeds.push(dx / dt);
        for _ in 0..pipe.sections {
            sec_pipe.push(index);
        }
    }
    let sections = sec_pipe.len();
    let nodes = sections + 1;
    // Each section's length, wave speed and area, the same numbers every step.
    let dx_table: Vec<f64> = (0..sections).map(|s| spec.pipes[sec_pipe[s]].length_m / spec.pipes[sec_pipe[s]].sections as f64).collect();
    let a_table: Vec<f64> = (0..sections).map(|s| run_speeds[sec_pipe[s]]).collect();
    let area_table: Vec<f64> = (0..sections).map(|s| area(&spec.pipes[sec_pipe[s]])).collect();
    let dx_of = |s: usize| dx_table[s];
    let a_of = |s: usize| a_table[s];
    let area_of = |s: usize| area_table[s];
    let z: Vec<f64> = (0..sections).map(|s| rho * a_of(s) / area_of(s)).collect();
    // Each node's distance from the supply: a pipe's start and a whole
    // number of its sections, so a pipe ends where its length says.
    let mut positions = vec![0.0f64; nodes];
    let mut node = 0usize;
    let mut start_m = 0.0f64;
    for pipe in &spec.pipes {
        for k in 1..=pipe.sections {
            node += 1;
            positions[node] = if k == pipe.sections { start_m + pipe.length_m } else { start_m + pipe.length_m * k as f64 / pipe.sections as f64 };
        }
        start_m += pipe.length_m;
    }
    let reach: Vec<f64> = (0..nodes)
        .map(|j| {
            let left = if j > 0 { area_of(j - 1) * dx_of(j - 1) } else { 0.0 };
            let right = if j < sections { area_of(j) * dx_of(j) } else { 0.0 };
            0.5 * (left + right)
        })
        .collect();
    let supply = spec.supply_pressure_pa;
    let cavities: Vec<Cavity> = (0..nodes)
        .map(|j| Cavity {
            gas_pa_m3: if j == 0 { 0.0 } else { spec.free_gas_fraction_at_atmosphere * ATMOSPHERE_PA * reach[j] },
            supply_pa: supply,
            vapour_pa: liquid.vapour_pressure_pa,
        })
        .collect();

    // The lumped stiffening, where the run carries one: every node but the
    // supply's holds the lump of its reach.
    let law = ext.stiffening.as_ref().map(|stiff| LumpLaw::new(stiff, supply));
    let lumps: Vec<Option<Lump>> = (0..nodes)
        .map(|j| match &law {
            Some(law) if j > 0 => Some(Lump { law, reach: reach[j], vapour_pa: liquid.vapour_pressure_pa }),
            _ => None,
        })
        .collect();
    let node_energy = |j: usize, v: f64| match &lumps[j] {
        Some(lump) => lump.energy(v),
        None => cavities[j].energy(v),
    };
    // The volume past which a node holds vapour: its lump at the vapour pressure.
    let vapour_from: Vec<f64> = (0..nodes).map(|j| lumps[j].map_or(0.0, |l| l.vapour_volume())).collect();
    let reference_energy: Vec<f64> = (0..nodes)
        .map(|j| match &lumps[j] {
            Some(lump) => lump.energy(lump.reach * lump.law.s_at(supply)),
            None => cavities[j].energy(cavities[j].gas_pa_m3 / supply),
        })
        .collect();
    let tables: Option<Vec<FrictionTable>> =
        if ext.friction_table { Some(spec.pipes.iter().map(|p| FrictionTable::new(p.roughness_m / p.diameter_m)).collect()) } else { None };
    // Steady state: one flow, each section losing its friction.
    let q0 = spec.initial_flow_m3_s;
    let mut steady_gauge = vec![0.0f64; nodes];
    for s in 0..sections {
        let pipe = &spec.pipes[sec_pipe[s]];
        let drop = match &tables {
            Some(t) => dx_of(s) * friction_per_volume_table(q0 / area_of(s), pipe, &liquid, &t[sec_pipe[s]]),
            None => dx_of(s) * friction_per_volume(q0 / area_of(s), pipe, &liquid),
        };
        steady_gauge[s + 1] = steady_gauge[s] - drop;
    }
    let steady: Vec<f64> = steady_gauge.iter().map(|g| g + supply).collect();
    let valve_steady = steady[nodes - 1];
    if q0 > 0.0 && valve_steady <= spec.return_pressure_pa {
        return Err(LineStop::SupplyCannotDriveFlow { valve_pressure_pa: valve_steady });
    }
    if let Some(low) = steady.iter().copied().find(|p| *p <= liquid.vapour_pressure_pa) {
        return Err(LineStop::SteadyBelowVapour { pressure_pa: low });
    }
    let valve_coefficient = if q0 > 0.0 { q0 / (2.0 * (valve_steady - spec.return_pressure_pa) / rho).sqrt() } else { 0.0 };
    let mut volume: Vec<f64> = (0..nodes)
        .map(|j| match &lumps[j] {
            Some(lump) => lump.reach * lump.law.s_at(steady[j]),
            None if cavities[j].gas_pa_m3 > 0.0 => cavities[j].gas_pa_m3 / steady[j],
            None => 0.0,
        })
        .collect();
    // Each node's volume at rest at the supply pressure, where it holds gas
    // or a lump: a node there whose flows are balanced at that pressure is
    // its own step's root, exactly, and keeps it.
    let rest_volume: Vec<f64> = (0..nodes)
        .map(|j| match &lumps[j] {
            Some(lump) => lump.reach * lump.law.s_at(supply),
            None if cavities[j].gas_pa_m3 > 0.0 => cavities[j].gas_pa_m3 / supply,
            None => f64::NAN,
        })
        .collect();
    // Waves departing at the steady state.
    let mut cp: Vec<f64> = (0..sections).map(|s| steady_gauge[s] + z[s] * q0).collect();
    let mut cm: Vec<f64> = (0..sections).map(|s| steady_gauge[s + 1] - z[s] * q0).collect();

    // Each lumped node's segment of its law and its energies, lump and
    // vapour, kept from step to step: the node's old energy is the one it
    // landed with, and the ledger sums them.
    let mut hints: Vec<usize> = (0..nodes).map(|j| lumps[j].map_or(0, |l| l.law.find_s(volume[j] / l.reach, 0))).collect();
    let mut node_energies: Vec<(f64, f64)> = (0..nodes)
        .map(|j| match &lumps[j] {
            Some(lump) => {
                let (p, _, k) = lump.state(volume[j], hints[j]);
                lump.energies_at(volume[j], p, k)
            }
            None => (0.0, 0.0),
        })
        .collect();
    // The ledger from the fields, with the sizes of the terms each sum adds:
    // energy (every section's waves, every node's terms), the waves' volume
    // and the nodes' volume.
    let ledger_of = |cp: &[f64], cm: &[f64], volume: &[f64], gone: &Ledger, cached: &[(f64, f64)]| -> (Ledger, f64, f64, f64) {
        let mut ledger = *gone;
        ledger.kinetic_j = 0.0;
        ledger.liquid_compression_j = 0.0;
        ledger.wall_strain_j = 0.0;
        ledger.stored_volume_m3 = 0.0;
        let mut energy_terms = 0.0f64;
        let mut volume_terms = 0.0f64;
        let mut node_volume_terms = 0.0f64;
        for s in 0..sections {
            let k = dt / (8.0 * z[s]);
            let share = spec.pipes[sec_pipe[s]].wall_share;
            let elastic = k * (cp[s] + cm[s]) * (cp[s] + cm[s]);
            let kinetic = k * (cp[s] - cm[s]) * (cp[s] - cm[s]);
            ledger.kinetic_j += kinetic;
            ledger.wall_strain_j += share * elastic;
            ledger.liquid_compression_j += (1.0 - share) * elastic;
            let stored = dt * (cp[s] + cm[s]) / (2.0 * z[s]);
            ledger.stored_volume_m3 += stored;
            energy_terms += kinetic + elastic;
            volume_terms += stored.abs();
        }
        ledger.cavity_j = 0.0;
        ledger.liquid_lump_j = 0.0;
        ledger.cavity_volume_m3 = 0.0;
        for j in 1..nodes {
            match &lumps[j] {
                Some(lump) => {
                    let (lumped, vapour) = cached[j];
                    if spec.tamper != Tamper::LumpEnergyDropped {
                        ledger.liquid_lump_j += lumped - reference_energy[j];
                    }
                    ledger.cavity_j += vapour;
                    energy_terms += lump.magnitude(volume[j], lumped, vapour) + reference_energy[j].abs();
                }
                None => {
                    let energy = cavities[j].energy(volume[j]);
                    ledger.cavity_j += energy - reference_energy[j];
                    energy_terms += cavities[j].magnitude(volume[j], energy) + reference_energy[j].abs();
                }
            }
            ledger.cavity_volume_m3 += volume[j];
            node_volume_terms += volume[j].abs();
        }
        (ledger, energy_terms, volume_terms, node_volume_terms)
    };
    let (start, start_energy_terms, start_wave_volume_terms, start_node_volume_terms) = ledger_of(&cp, &cm, &volume, &Ledger::default(), &node_energies);
    let start_volume_terms = start_wave_volume_terms + start_node_volume_terms;
    let length: f64 = spec.pipes.iter().map(|p| p.length_m).sum();
    let line_volume: f64 = spec.pipes.iter().map(|p| area(p) * p.length_m).sum();
    let a_min = run_speeds.iter().copied().fold(f64::INFINITY, f64::min);
    // With a stiffening the liquid is most compliant at the bottom of its law.
    let a_min = match &law {
        Some(law) => 1.0 / (rho * (1.0 / (rho * a_min * a_min) + law.e[0])).sqrt(),
        None => a_min,
    };
    let velocity0 = q0 / area_of(sections - 1);
    let energy_scale = line_volume * (0.5 * rho * velocity0 * velocity0 + math::powi(supply - liquid.vapour_pressure_pa, 2) / (2.0 * rho * a_min * a_min));
    let volume_scale = line_volume * ((supply - liquid.vapour_pressure_pa) / (rho * a_min * a_min) + spec.free_gas_fraction_at_atmosphere);

    let steps = (spec.duration_s / dt).ceil() as u64;
    let stride = ((steps as usize + 1).div_ceil(spec.samples.max(1))).max(1);
    let mid = nodes / 2;
    let mut out = LineRun {
        time_step_s: dt,
        steps,
        run_wave_speeds_m_s: run_speeds.clone(),
        node_positions_m: positions.clone(),
        steady_pressures_pa: steady.clone(),
        valve_coefficient_m2: valve_coefficient,
        sample_times_s: vec![0.0],
        valve_pressure_pa: vec![valve_steady],
        valve_flow_m3_s: vec![q0],
        midpoint_pressure_pa: vec![steady[mid]],
        valve_cavity_m3: vec![volume[nodes - 1]],
        ledger_samples: vec![start],
        ledger_start: start,
        ledger_end: start,
        valve_pressure_every_step_pa: vec![valve_steady],
        pressure_max_along_line_pa: steady.clone(),
        pressure_min_along_line_pa: steady.clone(),
        valve_pressure_max_pa: valve_steady,
        valve_pressure_max_time_s: 0.0,
        valve_pressure_min_pa: valve_steady,
        line_pressure_max_pa: steady.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        line_pressure_min_pa: steady.iter().copied().fold(f64::INFINITY, f64::min),
        cavity_volume_max_m3: volume.iter().copied().fold(0.0, f64::max),
        largest_cavity_over_reach: 0.0,
        largest_mach: 0.0,
        first_vapour_time_s: Vec::new(),
        first_vapour_position_m: Vec::new(),
        final_pressures_pa: steady.clone(),
        final_velocities_m_s: vec![velocity0; nodes],
        energy_scale_j: energy_scale,
        volume_scale_m3: volume_scale,
        largest_energy_residual: 0.0,
        largest_volume_residual: 0.0,
        friction_passes_max: 0,
        node_passes_max: 0,
        probe_pressures_pa: ext.probe_nodes.iter().map(|&j| vec![steady[j]]).collect(),
        valve_vapour_m3: vec![(volume[nodes - 1] - vapour_from[nodes - 1]).max(0.0)],
        vapour_volume_max_m3: 0.0,
    };
    let mut gone = Ledger::default();
    // The sizes of what the gone terms add, step by step; and of the lumped
    // nodes' second-law terms and volumes over every step, since each lump's
    // energy is taken at its Newton root, settled to a part in 1e12 of its
    // volume, and what that root leaves unsettled, like the rounding of its
    // large volume, is carried from step to step.
    let (mut valve_work_terms, mut cavity_loss_terms, mut supply_volume_terms, mut valve_volume_terms) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (mut lump_run_terms, mut lump_run_volume) = (0.0f64, 0.0f64);
    // Each node's carried gap: its energy was taken at its last root, not at
    // the volume it carried on, and the two differ by at most this.
    let mut last_root_gap = vec![0.0f64; nodes];
    let mut had_vapour = vec![false; nodes];
    let mut arrive_p = vec![0.0f64; sections];
    let mut last_delta: Vec<f64> = (0..sections).map(|s| steady_gauge[s] - steady_gauge[s + 1]).collect();
    let mut arrive_m = vec![0.0f64; sections];
    let mut pressure = steady.clone();
    let mut velocity = vec![velocity0; nodes];
    let _ = length;
    for n in 1..=steps {
        let time = n as f64 * dt;
        // Waves in flight lose their friction.
        for s in 0..sections {
            let pipe = &spec.pipes[sec_pipe[s]];
            let (delta, work, _, passes) = section_friction(cp[s], cm[s], z[s], pipe, &liquid, dx_of(s), a_of(s), last_delta[s], tables.as_ref().map(|t| &t[sec_pipe[s]]))
                .ok_or_else(|| LineStop::FrictionNotSettled { time_s: time, position_m: positions[s] + 0.5 * dx_of(s) })?;
            out.friction_passes_max = out.friction_passes_max.max(passes);
            gone.friction_loss_j += work;
            last_delta[s] = delta;
            let (delta, work) = if spec.tamper == Tamper::FrictionPushes { (-delta, -work) } else { (delta, work) };
            if work < 0.0 {
                return Err(LineStop::FrictionGaveEnergy { time_s: time, position_m: positions[s] + 0.5 * dx_of(s), work_j: work });
            }
            arrive_p[s] = cp[s] - delta;
            arrive_m[s] = cm[s] + delta;
        }
        // The supply holds its pressure.
        let q_in = -arrive_m[0] / z[0];
        gone.supply_volume_m3 += q_in * dt;
        supply_volume_terms += (q_in * dt).abs();
        cp[0] = -arrive_m[0];
        pressure[0] = supply;
        velocity[0] = q_in / area_of(0);
        // The valve's opening at this time.
        let tau = match &ext.valve_schedule {
            Some(schedule) => schedule.at(time),
            None if spec.closure_time_s > 0.0 => (1.0 - time / spec.closure_time_s).max(0.0),
            None => 0.0,
        };
        let opening = valve_coefficient * tau;
        let mut vapour_now = 0.0f64;
        // The sizes of the nodes' own second-law terms this step.
        let mut node_terms = 0.0f64;
        for j in 1..nodes {
            let kind = if j < sections {
                NodeKind::Inner { cp_in: arrive_p[j - 1], z_left: z[j - 1], cm_in: arrive_m[j], z_right: z[j] }
            } else {
                NodeKind::Valve { cp_in: arrive_p[j - 1], z_left: z[j - 1], opening_m2: opening, return_pa: spec.return_pressure_pa, density: rho }
            };
            let mut located = (0.0, 0.0, 0usize);
            // What the energy this step lands with is off the volume it
            // carries on, for the next step's second law.
            let mut carried = 0.0f64;
            // At rest at the supply pressure with its flows balanced there.
            let resting = volume[j] == rest_volume[j] && kind.net_in(0.0, supply) == 0.0;
            let step = match &lumps[j] {
                Some(lump) if resting => {
                    located = lump.state(volume[j], hints[j]);
                    NodeStep { volume: volume[j], gauge: 0.0, collapse_loss: 0.0, passes: 0, root_gap_j: 0.0 }
                }
                None if resting => NodeStep { volume: volume[j], gauge: 0.0, collapse_loss: 0.0, passes: 0, root_gap_j: 0.0 },
                Some(lump) if spec.tamper == Tamper::LumpForward => {
                    let gauge = lump.state(volume[j], hints[j]).0 - supply;
                    let new_volume = old_volume_less_inflow(&kind, volume[j], gauge, supply, dt);
                    located = lump.state(new_volume, hints[j]);
                    hints[j] = located.2;
                    NodeStep { volume: new_volume, gauge, collapse_loss: 0.0, passes: 1, root_gap_j: 0.0 }
                }
                Some(lump) => match lump_step(&kind, lump, volume[j], dt, &mut hints[j]) {
                    LumpStep::Settled { volume, gauge, passes, located: at, root_gap_j, carried_gap_j } => {
                        located = at;
                        hints[j] = at.2;
                        carried = carried_gap_j;
                        NodeStep { volume, gauge, collapse_loss: 0.0, passes, root_gap_j }
                    }
                    // The lump empties within the step: the node is at the
                    // top of its law, and the rest of the inflow raises its
                    // pressure against the sections alone, to the pressure
                    // at which the flows take what the lump held. Past the
                    // wall's yield the run stops there by name.
                    LumpStep::AboveTop => {
                        let gauge = kind.pressure_for_net_in(volume[j] / dt, supply);
                        located = lump.state(0.0, hints[j]);
                        hints[j] = located.2;
                        let root_gap_j = (located.0 - supply - gauge).max(0.0) * volume[j];
                        NodeStep { volume: 0.0, gauge, collapse_loss: 0.0, passes: 0, root_gap_j }
                    }
                    LumpStep::NotSettled => return Err(LineStop::NodeNotSettled { time_s: time, position_m: positions[j] }),
                },
                None => {
                    let forward = spec.tamper == Tamper::CavityForward && cavities[j].gas_pa_m3 > 0.0;
                    node_step(&kind, &cavities[j], volume[j], dt, forward).ok_or_else(|| LineStop::NodeNotSettled { time_s: time, position_m: positions[j] })?
                }
            };
            out.node_passes_max = out.node_passes_max.max(step.passes);
            let (old_energy, new_energy, terms) = match &lumps[j] {
                Some(lump) => {
                    let old = node_energies[j].0 + node_energies[j].1;
                    let old_terms = lump.magnitude(volume[j], node_energies[j].0, node_energies[j].1);
                    node_energies[j] = lump.energies_at(step.volume, located.0, located.2);
                    let new_terms = lump.magnitude(step.volume, node_energies[j].0, node_energies[j].1);
                    (old, node_energies[j].0 + node_energies[j].1, old_terms + new_terms)
                }
                None => {
                    let (old, new) = (node_energy(j, volume[j]), node_energy(j, step.volume));
                    (old, new, cavities[j].magnitude(volume[j], old) + cavities[j].magnitude(step.volume, new))
                }
            };
            let work_on_cavity = step.gauge * (volume[j] - step.volume);
            let scheme = work_on_cavity - (new_energy - old_energy) - step.collapse_loss;
            // The second law, to the arithmetic of the node's own terms and
            // of its root: the pressure times the volumes, the energies'
            // terms, the collapse.
            let own_terms = step.gauge.abs() * (volume[j] + step.volume) + terms + step.collapse_loss.abs();
            let allowed = (1.0e-9 * energy_scale).max(NODE_ROUNDING * own_terms) + step.root_gap_j + last_root_gap[j];
            if scheme < -allowed {
                return Err(LineStop::CavityGaveEnergy { time_s: time, position_m: positions[j], work_j: scheme });
            }
            last_root_gap[j] = carried;
            gone.cavity_loss_j += step.collapse_loss + scheme;
            cavity_loss_terms += (step.collapse_loss + scheme).abs();
            node_terms += own_terms;
            if lumps[j].is_some() {
                lump_run_terms += own_terms;
                lump_run_volume += step.volume;
            }
            let q_left = (arrive_p[j - 1] - step.gauge) / z[j - 1];
            cm[j - 1] = 2.0 * step.gauge - arrive_p[j - 1];
            let mut speed = (q_left / area_of(j - 1)).abs();
            if j < sections {
                let q_right = (step.gauge - arrive_m[j]) / z[j];
                cp[j] = 2.0 * step.gauge - arrive_m[j];
                if spec.tamper == Tamper::WaveGain {
                    cp[j] *= 1.0 + 1.0e-6;
                }
                speed = speed.max((q_right / area_of(j)).abs());
                velocity[j] = 0.5 * (q_left / area_of(j - 1) + q_right / area_of(j));
            } else {
                let q_valve = valve_flow(opening, step.gauge + supply - spec.return_pressure_pa, rho);
                if spec.tamper != Tamper::ValveWorkDropped {
                    gone.valve_work_j += step.gauge * q_valve * dt;
                }
                valve_work_terms += (step.gauge * q_valve * dt).abs();
                gone.valve_volume_m3 += q_valve * dt;
                valve_volume_terms += (q_valve * dt).abs();
                velocity[j] = q_left / area_of(j - 1);
                if (n as usize) % stride == 0 || n == steps {
                    out.valve_flow_m3_s.push(q_valve);
                }
            }
            let p = step.gauge + supply;
            volume[j] = step.volume;
            pressure[j] = p;
            if !p.is_finite() || !step.volume.is_finite() {
                return Err(LineStop::NonFiniteValue { time_s: time });
            }
            if p > spec.wall_yield_pressure_pa {
                return Err(LineStop::WallYield { time_s: time, position_m: positions[j], pressure_pa: p });
            }
            // The free gas's share of the node's volume at its pressure, by
            // the relation the question is asked by.
            if let Some(limit) = ext.void_fraction_limit {
                if cavities[j].gas_pa_m3 > 0.0 {
                    let void_fraction = void_fraction(spec.free_gas_fraction_at_atmosphere, p);
                    if void_fraction > limit {
                        return Err(LineStop::VoidAboveBubblyFlow { time_s: time, position_m: positions[j], void_fraction });
                    }
                }
            }
            // A lumped node holds vapour past its lump's volume at the vapour
            // pressure; one without, any cavity at or under it.
            let vapour_here = if lumps[j].is_some() { step.volume > vapour_from[j] } else { step.volume > 0.0 && p <= liquid.vapour_pressure_pa };
            if vapour_here {
                vapour_now += step.volume - vapour_from[j];
            }
            if vapour_here && !had_vapour[j] {
                had_vapour[j] = true;
                out.first_vapour_time_s.push(time);
                out.first_vapour_position_m.push(positions[j]);
            }
            if !vapour_here {
                // Local wave speed of the liquid with its gas at this pressure.
                let a_line = a_of(j - 1);
                let gas_compliance = match &lumps[j] {
                    Some(_) => located.1,
                    None if cavities[j].gas_pa_m3 > 0.0 => cavities[j].gas_pa_m3 / (p * p) / reach[j],
                    None => 0.0,
                };
                let local = 1.0 / (rho * (1.0 / (rho * a_line * a_line) + gas_compliance)).sqrt();
                let mach = speed / local;
                out.largest_mach = out.largest_mach.max(mach);
                if mach > spec.mach_limit {
                    return Err(LineStop::LineMachAboveAcoustic { time_s: time, position_m: positions[j], mach });
                }
            }
            // A cavity between sections may hold the oil it is lumped from;
            // at the valve, the column's end may move back one section.
            if ext.vapour_share_limit.is_none() {
                let room = if j < sections { reach[j] } else { area_of(j - 1) * dx_of(j - 1) };
                let over = step.volume / room;
                out.largest_cavity_over_reach = out.largest_cavity_over_reach.max(over);
                if over > 1.0 {
                    return Err(LineStop::CavityLargerThanReach { time_s: time, position_m: positions[j], cavity_over_reach: over });
                }
            }
            out.cavity_volume_max_m3 = out.cavity_volume_max_m3.max(step.volume);
            if p > out.pressure_max_along_line_pa[j] {
                out.pressure_max_along_line_pa[j] = p;
            }
            if p < out.pressure_min_along_line_pa[j] {
                out.pressure_min_along_line_pa[j] = p;
            }
        }
        out.vapour_volume_max_m3 = out.vapour_volume_max_m3.max(vapour_now);
        if let Some(share) = ext.vapour_share_limit {
            // The vapour of the whole line against the room it is given; the
            // place named is the valve's, where the column separates.
            let over = vapour_now / (share * line_volume);
            out.largest_cavity_over_reach = out.largest_cavity_over_reach.max(over);
            if over > 1.0 {
                return Err(LineStop::CavityLargerThanReach { time_s: time, position_m: positions[nodes - 1], cavity_over_reach: over });
            }
        }
        for (k, &j) in ext.probe_nodes.iter().enumerate() {
            out.probe_pressures_pa[k].push(pressure[j]);
        }
        // The ledgers, from the fields, against what has gone.
        let (now, energy_terms, wave_volume_terms, node_volume_terms) = ledger_of(&cp, &cm, &volume, &gone, &node_energies);
        // Each scale is at least SCALE_FLOOR of the sizes of the terms its
        // ledger adds at this step: the fields now and at the start, the
        // nodes' own second-law terms, and what has gone. Below that the
        // ledger would be judging the rounding of those terms, not the
        // physics.
        let energy_terms = energy_terms + node_terms + start_energy_terms + gone.friction_loss_j.abs() + valve_work_terms + cavity_loss_terms;
        let volume_terms = wave_volume_terms + node_volume_terms + start_volume_terms + supply_volume_terms + valve_volume_terms;
        let energy_scale_now = energy_scale.max(SCALE_FLOOR * energy_terms + RUN_FLOOR * lump_run_terms);
        let volume_scale_now = volume_scale.max(SCALE_FLOOR * volume_terms + RUN_FLOOR * lump_run_volume);
        let energy_residual = (now.stored_j() + now.gone_j() - start.stored_j()) / energy_scale_now;
        // The oil in the line is its geometric volume less the cavities, and
        // the stored volume more: what the supply put in less what the valve
        // let out.
        let volume_residual = (now.stored_volume_m3 - now.cavity_volume_m3 - now.supply_volume_m3 + now.valve_volume_m3
            - start.stored_volume_m3
            + start.cavity_volume_m3)
            / volume_scale_now;
        if !energy_residual.is_finite() || !volume_residual.is_finite() {
            return Err(LineStop::NonFiniteValue { time_s: time });
        }
        out.largest_energy_residual = out.largest_energy_residual.max(energy_residual.abs());
        out.largest_volume_residual = out.largest_volume_residual.max(volume_residual.abs());
        if energy_residual.abs() > ENERGY_RESIDUAL_LIMIT {
            return Err(LineStop::EnergyLedgerBroken { time_s: time, residual: energy_residual });
        }
        if volume_residual.abs() > VOLUME_RESIDUAL_LIMIT {
            return Err(LineStop::VolumeLedgerBroken { time_s: time, residual: volume_residual });
        }
        let valve_p = pressure[nodes - 1];
        out.valve_pressure_every_step_pa.push(valve_p);
        if valve_p > out.valve_pressure_max_pa {
            out.valve_pressure_max_pa = valve_p;
            out.valve_pressure_max_time_s = time;
        }
        out.valve_pressure_min_pa = out.valve_pressure_min_pa.min(valve_p);
        if (n as usize) % stride == 0 || n == steps {
            out.sample_times_s.push(time);
            out.valve_pressure_pa.push(valve_p);
            out.midpoint_pressure_pa.push(pressure[mid]);
            out.valve_cavity_m3.push(volume[nodes - 1]);
            out.valve_vapour_m3.push((volume[nodes - 1] - vapour_from[nodes - 1]).max(0.0));
            out.ledger_samples.push(now);
        }
        out.ledger_end = now;
    }
    out.line_pressure_max_pa = out.pressure_max_along_line_pa.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    out.line_pressure_min_pa = out.pressure_min_along_line_pa.iter().copied().fold(f64::INFINITY, f64::min);
    out.final_pressures_pa = pressure;
    out.final_velocities_m_s = velocity;
    Ok(out)
}

/// The energy ledger's residual, over the energy scale, held at every step.
pub const ENERGY_RESIDUAL_LIMIT: f64 = 1.0e-9;
/// The volume ledger's residual, over the volume scale, held at every step.
pub const VOLUME_RESIDUAL_LIMIT: f64 = 1.0e-9;
/// Each ledger's scale is at least this share of the sizes of the terms it
/// adds at a step: a sum of about a hundred terms rounds by at most a
/// hundred halves of 2.2e-16 of their sizes, 1.1e-14, a hundredth of 1e-9
/// of this share. The physical scale shrinks with the flow's speed and the
/// supply's height over the vapour pressure, while a node's gas or lump
/// keeps its own size: a line creeping near saturation would otherwise be
/// judged on the rounding of those terms, not on its physics.
pub const SCALE_FLOOR: f64 = 1.0e-5;
/// Each ledger's scale is also at least this share of the sizes of the
/// lumped nodes' terms, second-law terms for energy and volumes for volume,
/// summed over every step so far.
pub const RUN_FLOOR: f64 = 1.0e-6;
/// A node's second law is held to this share of the sizes of its own terms
/// (its pressure times its volumes, its energies' terms, a collapse), where
/// that is more than a part in 1e9 of the energy scale: sixteen units of the
/// last place.
pub const NODE_ROUNDING: f64 = 16.0 * f64::EPSILON;

/// The free gas's share of the volume at an absolute pressure: `g/(1+g)`
/// with `g` the gas per volume of liquid there, `alpha0 p_atm / p`; one
/// where `g` passes the 64-bit range.
pub fn void_fraction(free_gas_fraction_at_atmosphere: f64, pressure_pa: f64) -> f64 {
    let gas = free_gas_fraction_at_atmosphere * ATMOSPHERE_PA / pressure_pa;
    if gas.is_finite() { gas / (1.0 + gas) } else { 1.0 }
}

#[cfg(test)]
mod lump_tests {
    use super::*;

    fn law() -> LumpLaw {
        // An excess compliance falling from 6e-10 /Pa at 2 bar to zero at 250 bar.
        let pressure_pa: Vec<f64> = (0..=99).map(|k| 2.0e5 + k as f64 * 2.5e5).chain([250.0e5]).collect();
        let excess = pressure_pa.iter().map(|p| 6.0e-10 * (1.0 - (p - 2.0e5) / 248.0e5).powi(2)).collect();
        LumpLaw::new(&Stiffening { pressure_pa, excess_compliance_per_pa: excess }, 30.0e5)
    }

    /// The lump's volume and pressure are each other's inverse, and its
    /// stored energy rises by the gauge pressure times the volume it loses.
    #[test]
    fn the_lump_law_is_its_own_inverse_and_its_energy_is_the_work() {
        let law = law();
        let lump = Lump { law: &law, reach: 1.0e-5, vapour_pa: 2.0e5 };
        for p in [2.0e5, 3.3e5, 30.0e5, 87.6e5, 249.9e5] {
            let s = law.s_at(p);
            let (back, e, k) = law.locate(s, 0);
            assert!((back / p - 1.0).abs() < 1e-12, "{p} {back}");
            assert!((e - law.e[k] - law.slope[k] * (back - law.p[k])).abs() < 1e-24);
            // dU/db = p_s - p: a small step of volume about b.
            let b = lump.reach * s;
            let db = 1.0e-6 * b.max(1.0e-12);
            let (u_minus, u_plus) = (lump.energy(b - db), lump.energy(b + db));
            let slope = (u_plus - u_minus) / (2.0 * db);
            assert!((slope - (law.supply - p)).abs() < 1e-3 * (law.supply - p).abs().max(1.0e5), "{p} {slope}");
        }
        // At the supply the energy is zero, and it is convex in the volume.
        assert!(lump.energy(lump.reach * law.s_at(30.0e5)).abs() < 1e-12);
        let energies: Vec<f64> = (1..60).map(|k| lump.energy(lump.vapour_volume() * k as f64 / 50.0)).collect();
        assert!(energies.windows(3).all(|w| w[0] + w[2] - 2.0 * w[1] >= -1e-15));
    }
}
