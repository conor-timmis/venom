//! Small deterministic random helpers shared by generator components.
//!
//! `fastrand` owns the stream; these helpers express the generator's intent
//! (a percentage chance, a slice pick, a weighted pick) without scattering
//! conversion arithmetic through the call sites.

use fastrand::Rng;

pub(super) const PERCENT_MAX: u32 = 100;

pub(super) fn chance(rng: &mut Rng, percent: u32) -> bool {
    rng.u32(..PERCENT_MAX) < percent
}

pub(super) fn pick<'a, T>(rng: &mut Rng, values: &'a [T]) -> &'a T {
    debug_assert!(!values.is_empty(), "cannot pick from an empty collection");
    &values[pick_index(rng, values.len())]
}

pub(super) fn pick_index(rng: &mut Rng, len: usize) -> usize {
    debug_assert!(len > 0, "cannot pick from an empty collection");
    rng.u32(..len as u32) as usize
}

/// Returns an index from a non-empty weight table.
pub(super) fn weighted_index(rng: &mut Rng, weights: &[u32], total_weight: u32) -> usize {
    debug_assert!(!weights.is_empty(), "weights must not be empty");
    debug_assert!(total_weight > 0, "total weight must be positive");
    debug_assert!(
        weights.iter().all(|weight| *weight > 0),
        "zero-weight entries would make selection ambiguous"
    );

    let mut roll = rng.u32(..total_weight);
    for (index, weight) in weights.iter().enumerate() {
        if roll < *weight {
            return index;
        }
        roll -= *weight;
    }

    debug_assert!(false, "weights must cover every roll");
    0
}

#[cfg(test)]
mod tests {
    use fastrand::Rng;

    use super::{chance, pick_index, weighted_index};

    #[test]
    fn chance_handles_bounds() {
        let mut rng = Rng::with_seed(3);

        assert!(!chance(&mut rng, 0));
        assert!(chance(&mut rng, 100));
    }

    #[test]
    fn pick_index_stays_in_bounds() {
        let mut rng = Rng::with_seed(5);

        for _ in 0..1_000 {
            assert!(pick_index(&mut rng, 7) < 7);
        }
    }

    #[test]
    fn weighted_index_only_returns_positive_entries() {
        let mut rng = Rng::with_seed(13);
        let weights = [1, 3, 6];

        for _ in 0..1_000 {
            assert!(weighted_index(&mut rng, &weights, 10) < weights.len());
        }
    }
}
