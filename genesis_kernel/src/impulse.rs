//! The load rule of the cards that limit a load by what it adds in one step:
//! `|x| * dt_s <= C`, where `C` is a constant of the card.
//!
//! The rule is evaluated as it is written. The product `|x| * dt_s` is one
//! multiplication in 64-bit arithmetic, rounded once to the nearest 64-bit
//! number, with no fused multiply-add, and that product is compared with `C`.
//! The quotient `C / dt_s` is never the test: it is rounded too, and a load
//! just above it can have a product above `C`.

/// True when `|load| * dt_s <= limit`, the product rounded once. False when
/// the product is not a number.
#[must_use]
pub fn within(load: f64, dt_s: f64, limit: f64) -> bool {
    let product = load.abs() * dt_s;
    product <= limit
}

/// The largest load, in size, for which [`within`] is true: the number the
/// refusal sentences name. `None` when `limit` or `dt_s` is not a positive
/// finite number or no positive finite load qualifies.
#[must_use]
pub fn largest(limit: f64, dt_s: f64) -> Option<f64> {
    if !(dt_s.is_finite() && dt_s > 0.0 && limit.is_finite() && limit > 0.0) {
        return None;
    }
    let mut load = limit / dt_s;
    if !(load.is_finite() && load > 0.0) {
        return None;
    }
    // The quotient is within a few units in the last place of the answer.
    while !within(load, dt_s, limit) {
        let below = f64::from_bits(load.to_bits() - 1);
        if !(below > 0.0) {
            return None;
        }
        load = below;
    }
    loop {
        let above = f64::from_bits(load.to_bits() + 1);
        if above.is_finite() && within(above, dt_s, limit) {
            load = above;
        } else {
            break;
        }
    }
    Some(load)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn largest_is_the_last_load_within_and_the_next_is_not() {
        for (limit, scale) in [(239.99999999999997, 1.0e-6), (0.5, 3.0), (7.0e-4, 1.0e-3)] {
            for k in 0..4000 {
                let dt_s = scale * (1.0 + 0.003 * k as f64);
                let load = largest(limit, dt_s).unwrap();
                assert!(within(load, dt_s, limit), "{limit} {dt_s}");
                let above = f64::from_bits(load.to_bits() + 1);
                assert!(!within(above, dt_s, limit), "{limit} {dt_s}");
                assert!(within(-load, dt_s, limit));
            }
        }
        assert_eq!(largest(1.0, 0.0), None);
        assert_eq!(largest(f64::NAN, 1.0), None);
        assert!(!within(f64::NAN, 1.0, 1.0));
    }

    /// At some step sizes (454 of the 4000 here) the rounded quotient `C / dt_s`
    /// has a rounded product above `C`; `largest` is then the number below it, and
    /// the quotient is refused. At the rest the two are the same number or
    /// the quotient is the larger of the two that qualify.
    #[test]
    fn the_rounded_quotient_is_often_one_number_too_large() {
        let limit = 239.99999999999997;
        let mut over = 0;
        for k in 0..4000 {
            let dt_s = 1.0e-6 * 1.003_f64.powi(k);
            let quotient = limit / dt_s;
            let load = largest(limit, dt_s).unwrap();
            if !within(quotient, dt_s, limit) {
                over += 1;
                assert!(load < quotient);
            } else {
                assert!(load >= quotient);
            }
        }
        assert!(over > 100 && over < 1000, "{over}");
    }

    /// For several step sizes, the last load a card accepts and the first it
    /// refuses are the neighbours at which `|x| * dt_s <= limit` turns false,
    /// the product evaluated here, in the test, as the description writes it.
    /// `refused(x, dt_s)` is the card's own check.
    pub(crate) fn assert_boundary(limit: f64, refused: impl Fn(f64, f64) -> bool) {
        let mut steps = 0;
        for k in 0..60 {
            let dt_s = 1.0e-6 * 1.37_f64.powi(k) * (1.0 + 0.0123 * k as f64);
            if !(dt_s.is_finite() && dt_s > 0.0 && limit / dt_s > 0.0 && (limit / dt_s).is_finite()) {
                continue;
            }
            let centre = (limit / dt_s).to_bits();
            let mut last_accepted = None;
            let mut first_refused = None;
            for bits in (centre - 6)..=(centre + 6) {
                let load = f64::from_bits(bits);
                let published = !(load * dt_s <= limit);
                assert_eq!(refused(load, dt_s), published, "dt_s {dt_s:e} load {load:e}");
                assert_eq!(refused(-load, dt_s), published, "dt_s {dt_s:e} load -{load:e}");
                if published {
                    first_refused.get_or_insert(bits);
                } else {
                    last_accepted = Some(bits);
                }
            }
            let (accepted, refusal) = (last_accepted.unwrap(), first_refused.unwrap());
            assert_eq!(refusal, accepted + 1, "dt_s {dt_s:e}");
            assert_eq!(largest(limit, dt_s).unwrap().to_bits(), accepted, "dt_s {dt_s:e}");
            steps += 1;
        }
        assert!(steps >= 40, "{steps}");
    }
}
