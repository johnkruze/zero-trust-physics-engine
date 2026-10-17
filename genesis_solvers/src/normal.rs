//! The rule of a number below the normal range.
//!
//! A 64-bit number that is not zero and whose size is under 2^-1022
//! (2.2250738585072014e-308), the smallest normal number, is subnormal: it
//! holds fewer than 53 significant bits, and no answer publishes one (engine
//! invariant 4: every published number is finite, normal or zero). A solver
//! returns each value of its result below the normal range as zero with that
//! value's sign, `+0` or `-0`, so the result, its invariants and the field a
//! solution's `field_sha256` is taken over hold that zero. The ledgers are
//! closed on the values as computed; the change is under 2^-1022 in size, far
//! below every solver's ledger tolerance.
//!
//! A normal number, a zero, an infinity and NaN are kept as they are: a value
//! that is not finite is refused by name where it is computed.

/// The sign bit of a 64-bit number.
const SIGN_BIT: u64 = 0x8000_0000_0000_0000;

/// `value`, or zero with its sign when it is subnormal.
#[must_use]
pub const fn normal_or_zero(value: f64) -> f64 {
    if value.is_subnormal() { f64::from_bits(value.to_bits() & SIGN_BIT) } else { value }
}

/// A result held to the rule: every 64-bit number in it, at every depth,
/// written by [`normal_or_zero`]. Counts, flags and words are left alone.
pub trait NormalOrZero {
    /// Write each number below the normal range as zero with its sign, in place.
    fn normal_or_zero(&mut self);
}

impl NormalOrZero for f64 {
    fn normal_or_zero(&mut self) {
        *self = normal_or_zero(*self);
    }
}

impl<T: NormalOrZero> NormalOrZero for [T] {
    fn normal_or_zero(&mut self) {
        for value in self.iter_mut() {
            value.normal_or_zero();
        }
    }
}

impl<T: NormalOrZero, const N: usize> NormalOrZero for [T; N] {
    fn normal_or_zero(&mut self) {
        self.as_mut_slice().normal_or_zero();
    }
}

impl<T: NormalOrZero> NormalOrZero for Vec<T> {
    fn normal_or_zero(&mut self) {
        self.as_mut_slice().normal_or_zero();
    }
}

impl<T: NormalOrZero> NormalOrZero for Option<T> {
    fn normal_or_zero(&mut self) {
        if let Some(value) = self {
            value.normal_or_zero();
        }
    }
}

impl<A: NormalOrZero, B: NormalOrZero> NormalOrZero for (A, B) {
    fn normal_or_zero(&mut self) {
        self.0.normal_or_zero();
        self.1.normal_or_zero();
    }
}

/// `answer` with the rule applied: what a solver's public `solve` returns.
pub fn held<T: NormalOrZero, E>(answer: Result<T, E>) -> Result<T, E> {
    answer.map(|mut answer| {
        answer.normal_or_zero();
        answer
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest normal number and every number above it in size are kept
    /// bit for bit; a subnormal is zero with its sign; zero, minus zero, the
    /// infinities and NaN are kept.
    #[test]
    fn a_subnormal_is_zero_with_its_sign_and_every_other_number_is_kept() {
        assert_eq!(f64::MIN_POSITIVE, 2.2250738585072014e-308);
        for kept in [f64::MIN_POSITIVE, -f64::MIN_POSITIVE, 1.0, -1.0e-300, f64::MAX, 0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(normal_or_zero(kept).to_bits(), kept.to_bits(), "{kept:e}");
        }
        assert!(normal_or_zero(f64::NAN).is_nan());
        let largest_subnormal = f64::from_bits(f64::MIN_POSITIVE.to_bits() - 1);
        for (subnormal, zero) in [(largest_subnormal, 0.0_f64), (-largest_subnormal, -0.0), (5.0e-324, 0.0), (-2.06014874156e-313, -0.0)] {
            assert!(subnormal.is_subnormal());
            assert_eq!(normal_or_zero(subnormal).to_bits(), zero.to_bits(), "{subnormal:e}");
        }
    }

    /// Every depth of a result is held: lists, arrays, lists of arrays,
    /// options and pairs.
    #[test]
    fn every_depth_of_a_result_is_held() {
        let mut nested: (Vec<[f64; 2]>, Option<Vec<f64>>) = (vec![[1.0, 5.0e-324], [-1.0e-310, 2.0]], Some(vec![-5.0e-324, f64::MIN_POSITIVE]));
        nested.normal_or_zero();
        let bits = |values: &[f64]| values.iter().map(|value| value.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&nested.0[0]), bits(&[1.0, 0.0]));
        assert_eq!(bits(&nested.0[1]), bits(&[-0.0, 2.0]));
        assert_eq!(bits(nested.1.as_ref().unwrap()), bits(&[-0.0, f64::MIN_POSITIVE]));
        let refused: Result<f64, &str> = Err("refused");
        assert_eq!(held(refused), Err("refused"));
        assert_eq!(held::<f64, ()>(Ok(-1.0e-320)).unwrap().to_bits(), (-0.0_f64).to_bits());
    }
}
