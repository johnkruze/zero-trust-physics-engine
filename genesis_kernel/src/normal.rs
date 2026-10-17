//! The rule of a number below the normal range.
//!
//! A 64-bit number that is not zero and whose size is under 2^-1022
//! (2.2250738585072014e-308), the smallest normal number, is subnormal: it
//! holds fewer than 53 significant bits, and no answer publishes one (engine
//! invariant 4: every published number is finite, normal or zero). A card's
//! `step`, once the step has landed and its ledger has closed, writes each
//! value of the landed state below the normal range as zero with that value's
//! sign, `+0` or `-0`. The state carried to the next step, the frame and the
//! answer hold that zero, and the next step's ledger is closed on it. The
//! change is under 2^-1022 in size, far below every card's ledger allowance;
//! a frame slot, a 32-bit number, held zero for every such value before.
//!
//! A normal number, a zero, an infinity and NaN are returned as they are: a
//! value that is not finite is refused by name where it is computed.

/// The sign bit of a 64-bit number.
const SIGN_BIT: u64 = 0x8000_0000_0000_0000;

/// `value`, or zero with its sign when it is subnormal.
#[must_use]
pub const fn normal_or_zero(value: f64) -> f64 {
    if value.is_subnormal() { f64::from_bits(value.to_bits() & SIGN_BIT) } else { value }
}

/// Each value of `values` by [`normal_or_zero`], in place.
pub fn normal_or_zero_each<const N: usize>(values: &mut [f64; N]) {
    let mut index = 0;
    while index < N {
        values[index] = normal_or_zero(values[index]);
        index += 1;
    }
}

/// Each value of each row of `rows` by [`normal_or_zero`], in place.
pub fn normal_or_zero_rows<const N: usize, const M: usize>(rows: &mut [[f64; N]; M]) {
    let mut row = 0;
    while row < M {
        normal_or_zero_each(&mut rows[row]);
        row += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest normal number and every number above it in size are
    /// kept bit for bit; the largest subnormal, the smallest and one between
    /// are zero with their sign; zero, minus zero, the infinities and NaN are
    /// kept.
    #[test]
    fn a_subnormal_is_zero_with_its_sign_and_every_other_number_is_kept() {
        let smallest_normal = f64::MIN_POSITIVE;
        assert_eq!(smallest_normal, 2.2250738585072014e-308);
        for kept in [smallest_normal, -smallest_normal, 1.0, -1.0, 1.0e-300, f64::MAX, f64::MIN, 0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(normal_or_zero(kept).to_bits(), kept.to_bits(), "{kept:e}");
        }
        assert!(normal_or_zero(f64::NAN).is_nan());
        let largest_subnormal = f64::from_bits(smallest_normal.to_bits() - 1);
        for (subnormal, zero) in [
            (largest_subnormal, 0.0_f64),
            (-largest_subnormal, -0.0),
            (5.0e-324, 0.0),
            (-5.0e-324, -0.0),
            (5.959947000725654e-309, 0.0),
            (-9.7888843819865e-311, -0.0),
        ] {
            assert!(subnormal.is_subnormal(), "{subnormal:e}");
            assert_eq!(normal_or_zero(subnormal).to_bits(), zero.to_bits(), "{subnormal:e}");
        }
    }

    /// The arrays are held element by element, and the change is under the
    /// smallest normal number in size.
    #[test]
    fn arrays_are_held_element_by_element() {
        let mut values = [1.0, 5.0e-324, -1.0e-310, f64::MIN_POSITIVE];
        let before = values;
        normal_or_zero_each(&mut values);
        assert_eq!(values.map(f64::to_bits), [1.0_f64, 0.0, -0.0, f64::MIN_POSITIVE].map(f64::to_bits));
        for (a, b) in before.iter().zip(values) {
            assert!((a - b).abs() < f64::MIN_POSITIVE);
        }
        let mut rows = [[1.0e-320, 2.0], [-3.0, -4.0e-315]];
        normal_or_zero_rows(&mut rows);
        assert_eq!(rows.map(|row| row.map(f64::to_bits)), [[0.0_f64, 2.0], [-3.0, -0.0]].map(|row| row.map(f64::to_bits)));
    }
}
