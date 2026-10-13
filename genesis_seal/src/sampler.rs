//! The sampler `splitmix64_counter_v1`.
//!
//! A draw is a function of the seed, the row and the draw's position, so row
//! `i` is worked out without the rows before it. All arithmetic on 64-bit
//! whole numbers wraps.
//!
//! The uniform draw holds the only floating-point arithmetic in this crate.

/// The one generator a sample plan may name.
pub const GENERATOR: &str = "splitmix64_counter_v1";

const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// 2^-53.
const TWO_TO_MINUS_53: f64 = f64::from_bits(0x3CA0_0000_0000_0000);

pub fn mix(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The key of a row. `row` counts from 0.
pub fn row_key(seed: u64, row: u64) -> u64 {
    mix(seed.wrapping_add(GOLDEN.wrapping_mul(row.wrapping_add(1))))
}

/// The 64 bits `x` of draw `draw` of row `row`. `draw` is the draw's position
/// in the plan's `draws`, from 0.
pub fn draw_bits(seed: u64, row: u64, draw: u64) -> u64 {
    mix(row_key(seed, row).wrapping_add(GOLDEN.wrapping_mul(draw.wrapping_add(1))))
}

/// `k`, the top 53 bits of `x`: a whole number below 2^53.
pub fn whole_number(x: u64) -> u64 {
    x >> 11
}

/// `low + (k * 2^-53) * (high - low)`.
///
/// `k * 2^-53` is exact, because `k` is below 2^53. Three operations follow
/// in this order, each rounded to nearest even: the subtraction, the
/// multiplication and the addition. They are written as separate statements
/// and `mul_add` is not called, so no multiplication is fused with an
/// addition. The result is never below `low` and never above `high`.
pub fn uniform(k: u64, low: f64, high: f64) -> f64 {
    let unit = (k as f64) * TWO_TO_MINUS_53;
    let width = width(low, high);
    let offset = unit * width;
    low + offset
}

/// `high - low`, the subtraction of [`uniform`]. It is written once, here. A
/// plan whose width is not finite is refused before a row is drawn.
pub fn width(low: f64, high: f64) -> f64 {
    high - low
}

/// `(k * n) >> 53` in 128-bit whole-number arithmetic: an index below `n`.
pub fn choice_index(k: u64, n: u64) -> u64 {
    ((u128::from(k) * u128::from(n)) >> 53) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The published vectors. `outside_check.py` rebuilds the same file from
    /// the design document with no code of this crate.
    const VECTORS: &str = include_str!("../vectors/sampler.txt");

    fn word(text: &str) -> u64 {
        u64::from_str_radix(text, 16).expect("16 hex digits")
    }

    #[test]
    fn the_published_uniform_vectors_are_reproduced_bit_for_bit() {
        let mut checked = 0;
        for line in VECTORS.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.first() != Some(&"uniform") {
                continue;
            }
            let (seed, row, draw) = (word(parts[1]), parts[2].parse().unwrap(), parts[3].parse().unwrap());
            let x = draw_bits(seed, row, draw);
            let k = whole_number(x);
            assert_eq!(format!("{x:016x}"), parts[4], "{line}");
            assert_eq!(format!("{k:016x}"), parts[5], "{line}");
            let low = f64::from_bits(word(parts[6]));
            let high = f64::from_bits(word(parts[7]));
            let value = uniform(k, low, high);
            assert_eq!(format!("{:016x}", value.to_bits()), parts[8], "{line}");
            checked += 1;
        }
        assert_eq!(checked, 40);
    }

    #[test]
    fn the_published_choice_vectors_are_reproduced() {
        let mut checked = 0;
        for line in VECTORS.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.first() != Some(&"choice") {
                continue;
            }
            let (seed, row, draw) = (word(parts[1]), parts[2].parse().unwrap(), parts[3].parse().unwrap());
            let k = whole_number(draw_bits(seed, row, draw));
            assert_eq!(format!("{k:016x}"), parts[4], "{line}");
            let n: u64 = parts[5].parse().unwrap();
            assert_eq!(choice_index(k, n).to_string(), parts[6], "{line}");
            checked += 1;
        }
        assert_eq!(checked, 24);
    }

    #[test]
    fn the_mix_matches_the_published_splitmix64_outputs() {
        // SplitMix64 from state 0 steps its state by the golden constant and
        // mixes it. These three outputs are the widely published ones.
        assert_eq!(mix(GOLDEN), 0xE220_A839_7B1D_CDAF);
        assert_eq!(mix(GOLDEN.wrapping_mul(2)), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(mix(GOLDEN.wrapping_mul(3)), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn a_uniform_draw_starts_at_low_and_never_passes_high() {
        let largest = (1_u64 << 53) - 1;
        for (low, high) in [
            (0.0, 1.0),
            (233.15, 398.15),
            (-1.0, 1.0),
            (-3.0e-7, 2.5e9),
            (1.0, 1.0000000000000002),
            (0.1, 0.30000000000000004),
            (5.0, 5.0),
        ] {
            assert_eq!(uniform(0, low, high).to_bits(), f64::to_bits(low));
            for k in [1, 2, largest / 3, largest / 2, largest - 1, largest] {
                let value = uniform(k, low, high);
                assert!(value >= low && value <= high, "{low} {high} {k} {value}");
            }
        }
    }

    #[test]
    fn a_choice_index_is_below_the_count_of_choices() {
        let largest = (1_u64 << 53) - 1;
        for n in [1, 2, 3, 7, 1000, u64::MAX] {
            assert_eq!(choice_index(0, n), 0);
            assert!(choice_index(largest, n) < n);
        }
        assert_eq!(choice_index(largest, 2), 1);
        assert_eq!(choice_index(1 << 52, 2), 1);
        assert_eq!(choice_index((1 << 52) - 1, 2), 0);
    }

    #[test]
    fn a_row_and_a_draw_each_change_the_bits() {
        let seed = 0x0123_4567_89ab_cdef;
        assert_ne!(draw_bits(seed, 0, 0), draw_bits(seed, 1, 0));
        assert_ne!(draw_bits(seed, 0, 0), draw_bits(seed, 0, 1));
        assert_ne!(draw_bits(seed, 0, 1), draw_bits(seed, 1, 0));
        assert_ne!(draw_bits(seed, 0, 0), draw_bits(seed + 1, 0, 0));
        assert_eq!(draw_bits(seed, u64::MAX, u64::MAX), mix(mix(seed)));
    }
}
