//! Display/input units only. Authoritative local positions remain integer kilometers.
pub const KM_PER_AU: f64 = 149_597_870.7;

pub fn kilometers_to_au(kilometers: f64) -> f64 {
    kilometers / KM_PER_AU
}

pub fn au_to_kilometers(au: f64) -> Option<i64> {
    let kilometers = (au * KM_PER_AU).round();
    if !kilometers.is_finite() || kilometers < i64::MIN as f64 || kilometers >= i64::MAX as f64 {
        None
    } else {
        Some(kilometers as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::{KM_PER_AU, au_to_kilometers, kilometers_to_au};
    #[test]
    fn local_au_inputs_round_to_kilometers_and_reject_non_finite_or_overflowing_values() {
        for au in [0.0, 1.0, -0.5, 0.333485, 10.0] {
            let kilometers = au_to_kilometers(au).unwrap();
            assert!((kilometers as f64 - au * KM_PER_AU).abs() <= 0.5);
            assert!((kilometers_to_au(kilometers as f64) - au).abs() <= 0.5 / KM_PER_AU);
        }
        for au in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
            assert!(au_to_kilometers(au).is_none());
        }
    }
}
