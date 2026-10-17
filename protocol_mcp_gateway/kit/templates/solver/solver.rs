//! {{Title}}: the company's own specimen, a field refined until it settles.
//!
//! STAMPED FROM THE KIT TEMPLATE. The physics below is the template's: steady
//! heat conduction along a rod of one conductivity, its two ends held at two
//! temperatures, heat generated uniformly inside it. Replace it with the
//! solver's own, keeping the shape: constants, an enum of named limits with a
//! sentence above each variant, a result struct, one public function, tests.
//! Then remove this paragraph.
//!
//! The source of every number is the company's own derivation. The field is
//! the temperature at `CELLS` points along the rod, settled by successive
//! over-relaxation of
//!
//! ```text
//! k d2T/dx2 + q = 0,  T(0) = T_left,  T(L) = T_right
//! ```
//!
//! What must hold: at acceptance the heat leaving through the two ends equals
//! the heat generated inside, to `ENERGY_BALANCE_TOLERANCE` of it; on every
//! sweep the field stays finite. Its limits are physical: a temperature
//! anywhere at or below absolute zero, or above the melting point of the
//! rod.

/// The rod's length.
pub const ROD_LENGTH_M: f64 = 0.5;
/// Its thermal conductivity, aluminium near room temperature.
pub const CONDUCTIVITY_W_PER_M_K: f64 = 237.0;
/// The melting point of the rod. Above it the rod is not the solid this
/// equation describes.
pub const MELTING_POINT_K: f64 = 933.47;
/// Interior points of the field.
pub const CELLS: usize = 63;
/// Over-relaxation weight.
pub const RELAXATION_WEIGHT: f64 = 1.9;
/// A sweep that moves no point by more than this fraction of the field's
/// largest temperature settles the field: a few units in the last place.
pub const SETTLE_TOLERANCE: f64 = 1.0e-13;
/// The ends' heat over the heat generated, less one, at acceptance.
pub const ENERGY_BALANCE_TOLERANCE: f64 = 1.0e-9;
/// The most sweeps.
pub const MAX_SWEEPS: u32 = 100_000;

/// What the caller asks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct {{Name}}Question {
    pub left_temperature_k: f64,
    pub right_temperature_k: f64,
    pub heat_generation_w_per_m3: f64,
}

/// The settled field.
#[derive(Clone, Debug, PartialEq)]
pub struct {{Name}}Field {
    pub peak_temperature_k: f64,
    pub heat_out_left_w_per_m2: f64,
    pub heat_out_right_w_per_m2: f64,
    /// The ends' heat over the heat generated, less one; zero with no heat generated.
    pub energy_balance_residual: f64,
    pub sweeps: u32,
    /// The temperature at every point, the two ends included.
    pub temperature_k: Vec<f64>,
}

/// Why no field is returned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum {{Name}}Reject {
    /// An input or an intermediate value was NaN or infinite.
    NonFiniteValue,
    /// An end is at or below absolute zero, or above the melting point of the rod.
    EndOutsideSolid,
    /// The settled field is above the melting point of the rod somewhere.
    FieldAboveMeltingPoint,
    /// The settled field reaches absolute zero.
    FieldBelowAbsoluteZero,
    /// The sweep cap was reached with the field still moving.
    NotSettled,
    /// The settled field's ends do not carry the heat generated inside.
    EnergyBalanceBroken,
}

impl {{Name}}Reject {
    pub fn name(&self) -> &'static str {
        match self {
            Self::NonFiniteValue => "NonFiniteValue",
            Self::EndOutsideSolid => "EndOutsideSolid",
            Self::FieldAboveMeltingPoint => "FieldAboveMeltingPoint",
            Self::FieldBelowAbsoluteZero => "FieldBelowAbsoluteZero",
            Self::NotSettled => "NotSettled",
            Self::EnergyBalanceBroken => "EnergyBalanceBroken",
        }
    }
}

impl core::fmt::Display for {{Name}}Reject {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sentence = match self {
            Self::NonFiniteValue => "an input or an intermediate value was not finite",
            Self::EndOutsideSolid => "an end of the rod is at or below absolute zero, or above its melting point, 933.47 K",
            Self::FieldAboveMeltingPoint => "the settled field is above the rod's melting point, 933.47 K, somewhere",
            Self::FieldBelowAbsoluteZero => "the settled field reaches absolute zero",
            Self::NotSettled => "the field was still moving at the sweep cap",
            Self::EnergyBalanceBroken => "the settled field's ends do not carry the heat generated inside",
        };
        f.write_str(sentence)
    }
}

/// The settled field, or the limit that stopped it.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &{{Name}}Question) -> Result<{{Name}}Field, {{Name}}Reject> {
    crate::normal::held(solve_with(question, MAX_SWEEPS))
}

/// Every 64-bit number of the field, held to the rule; a field added to the
/// answer is added here.
impl crate::normal::NormalOrZero for {{Name}}Field {
    fn normal_or_zero(&mut self) {
        self.peak_temperature_k.normal_or_zero();
        self.heat_out_left_w_per_m2.normal_or_zero();
        self.heat_out_right_w_per_m2.normal_or_zero();
        self.energy_balance_residual.normal_or_zero();
        self.temperature_k.normal_or_zero();
    }
}

fn solve_with(question: &{{Name}}Question, max_sweeps: u32) -> Result<{{Name}}Field, {{Name}}Reject> {
    let {{Name}}Question { left_temperature_k: left, right_temperature_k: right, heat_generation_w_per_m3: q } = *question;
    if !(left.is_finite() && right.is_finite() && q.is_finite()) {
        return Err({{Name}}Reject::NonFiniteValue);
    }
    if !(left > 0.0 && right > 0.0 && left <= MELTING_POINT_K && right <= MELTING_POINT_K) {
        return Err({{Name}}Reject::EndOutsideSolid);
    }
    let points = CELLS + 2;
    let dx = ROD_LENGTH_M / (CELLS + 1) as f64;
    let source = q * dx * dx / CONDUCTIVITY_W_PER_M_K;
    let mut t: Vec<f64> = (0..points).map(|i| left + (right - left) * i as f64 / (points - 1) as f64).collect();
    let mut sweeps = 0;
    loop {
        sweeps += 1;
        let mut largest = 0.0_f64;
        let mut scale = 0.0_f64;
        for i in 1..points - 1 {
            let target = 0.5 * (t[i - 1] + t[i + 1] + source);
            let moved = RELAXATION_WEIGHT * (target - t[i]);
            t[i] += moved;
            largest = largest.max(moved.abs());
            scale = scale.max(t[i].abs());
        }
        if !(largest.is_finite() && scale.is_finite()) {
            return Err({{Name}}Reject::NonFiniteValue);
        }
        if largest <= SETTLE_TOLERANCE * scale.max(left).max(right) {
            break;
        }
        if sweeps >= max_sweeps {
            return Err({{Name}}Reject::NotSettled);
        }
    }
    // The heat leaving through each end, from the settled field by
    // second-order one-sided differences, exact on the parabola, against the
    // heat generated inside: a balance per square metre of the rod's section.
    let out_left = CONDUCTIVITY_W_PER_M_K * (-3.0 * t[0] + 4.0 * t[1] - t[2]) / (2.0 * dx);
    let out_right = -CONDUCTIVITY_W_PER_M_K * (3.0 * t[points - 1] - 4.0 * t[points - 2] + t[points - 3]) / (2.0 * dx);
    let generated = q * ROD_LENGTH_M;
    let residual = if generated == 0.0 {
        (out_left + out_right).abs() / (CONDUCTIVITY_W_PER_M_K * ((left - right).abs() + 1.0) / ROD_LENGTH_M)
    } else {
        ((out_left + out_right) / generated - 1.0).abs()
    };
    if !residual.is_finite() {
        return Err({{Name}}Reject::NonFiniteValue);
    }
    if residual > ENERGY_BALANCE_TOLERANCE {
        return Err({{Name}}Reject::EnergyBalanceBroken);
    }
    let lowest = t.iter().copied().fold(f64::MAX, f64::min);
    if !(lowest > 0.0) {
        return Err({{Name}}Reject::FieldBelowAbsoluteZero);
    }
    let peak = t.iter().copied().fold(f64::MIN, f64::max);
    if !(peak <= MELTING_POINT_K) {
        return Err({{Name}}Reject::FieldAboveMeltingPoint);
    }
    Ok({{Name}}Field {
        peak_temperature_k: peak,
        heat_out_left_w_per_m2: out_left,
        heat_out_right_w_per_m2: out_right,
        energy_balance_residual: residual,
        sweeps,
        temperature_k: t,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question() -> {{Name}}Question {
        {{Name}}Question { left_temperature_k: 300.0, right_temperature_k: 350.0, heat_generation_w_per_m3: 1.0e5 }
    }

    /// The company's own derivation: the exact field is a parabola, which the
    /// three-point difference holds exactly, so the settled field meets it to
    /// the settle tolerance over the relaxation's contraction.
    #[test]
    fn the_settled_field_is_the_parabola_of_its_derivation() {
        let field = solve(&question()).unwrap();
        let q = question();
        let n = field.temperature_k.len();
        for (i, t) in field.temperature_k.iter().enumerate() {
            let x = ROD_LENGTH_M * i as f64 / (n - 1) as f64;
            let exact = q.left_temperature_k
                + (q.right_temperature_k - q.left_temperature_k) * x / ROD_LENGTH_M
                + q.heat_generation_w_per_m3 / (2.0 * CONDUCTIVITY_W_PER_M_K) * x * (ROD_LENGTH_M - x);
            assert!((t - exact).abs() < 1.0e-8, "{i}: {t} against {exact}");
        }
        assert!(field.energy_balance_residual <= ENERGY_BALANCE_TOLERANCE);
    }

    #[test]
    fn each_limit_is_named() {
        let mut q = question();
        q.left_temperature_k = 0.0;
        assert_eq!(solve(&q), Err({{Name}}Reject::EndOutsideSolid));
        q.left_temperature_k = f64::from_bits(MELTING_POINT_K.to_bits() + 1);
        assert_eq!(solve(&q), Err({{Name}}Reject::EndOutsideSolid));
        q.left_temperature_k = f64::NAN;
        assert_eq!(solve(&q), Err({{Name}}Reject::NonFiniteValue));
        let hot = {{Name}}Question { heat_generation_w_per_m3: 1.0e8, ..question() };
        assert_eq!(solve(&hot), Err({{Name}}Reject::FieldAboveMeltingPoint));
        let cold = {{Name}}Question { heat_generation_w_per_m3: -1.0e8, ..question() };
        assert_eq!(solve(&cold), Err({{Name}}Reject::FieldBelowAbsoluteZero));
    }

    /// Rule 8: a field stopped before it settles is named, never returned.
    #[test]
    fn a_field_cut_short_is_named_not_settled() {
        assert_eq!(solve_with(&question(), 3), Err({{Name}}Reject::NotSettled));
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        assert_eq!(solve(&question()), solve(&question()));
    }
}
