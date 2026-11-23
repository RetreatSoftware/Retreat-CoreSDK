//! Seeded, deterministic random numbers.
//!
//! Contract: the same seed always produces the same sequence, on every platform
//! (native and `wasm32-unknown-unknown`) and in every version of this crate.
//! Command scripts, replays and native-vs-WASM determinism checks rely on it.
//! Changing the output of an existing seed is a breaking change.
//!
//! Not for security: the sequence is predictable by design.
//!
//! ```
//! use retreat_rng::RetreatRng;
//!
//! let mut a = RetreatRng::from_seed(42);
//! let mut b = RetreatRng::from_seed(42);
//! assert_eq!(a.next_u64(), b.next_u64());
//! ```

/// SplitMix64 generator (Steele, Lea, Flood 2014; reference: <https://prng.di.unimi.it/splitmix64.c>).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetreatRng {
    state: u64,
}

/// Added to the state on every step (the "golden gamma" of SplitMix64).
const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// `next_f32` keeps this many high bits: an `f32` has a 24-bit significand, so every
/// value `k / 2^24` is exact and the result is uniform in `[0, 1)`.
const F32_RANDOM_BITS: u32 = 24;

impl RetreatRng {
    /// Creates a generator. Every seed, including 0, is valid.
    pub fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next 64 random bits. Advances the generator by one step.
    pub fn next_u64(&mut self) -> u64 {
        // Wrapping arithmetic is part of the algorithm: overflow must wrap around,
        // not panic (debug builds panic on plain `+` / `*` overflow).
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform float in `[0.0, 1.0)`: never returns 1.0. Advances the generator by one step.
    pub fn next_f32(&mut self) -> f32 {
        let high_bits = self.next_u64() >> (64 - F32_RANDOM_BITS);
        high_bits as f32 / (1u32 << F32_RANDOM_BITS) as f32
    }

    /// Uniform float in `[min, max)`. Advances the generator by one step.
    ///
    /// # Panics
    /// If `min > max`, or if either bound is not finite (NaN or infinity).
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        assert!(
            min.is_finite() && max.is_finite() && min <= max,
            "RetreatRng::range_f32: invalid range [{min}, {max}); \
             both bounds must be finite and min <= max (swap the arguments if they are reversed)"
        );
        scale_unit_to_half_open_range(self.next_f32(), min, max)
    }
}

/// Maps `unit` in `[0, 1)` to `[min, max)`.
///
/// Float rounding can turn `min + (max - min) * unit` into exactly `max` (for example
/// `[100, 101)` with the largest `unit` below 1). That result is replaced by the largest
/// `f32` below `max`, so the upper bound stays exclusive. When `min == max`, returns `min`.
fn scale_unit_to_half_open_range(unit: f32, min: f32, max: f32) -> f32 {
    let value = min + (max - min) * unit;
    if value >= max && min < max {
        max.next_down()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many values the statistical-range tests draw.
    const SAMPLE_COUNT: usize = 10_000;

    #[test]
    fn matches_splitmix64_reference_output() {
        // First outputs of the reference C implementation for seed 1234567.
        let expected: [u64; 5] = [
            6457827717110365317,
            3203168211198807973,
            9817491932198370423,
            4593380528125082431,
            16408922859458223821,
        ];
        let mut rng = RetreatRng::from_seed(1234567);
        for (index, expected_value) in expected.iter().enumerate() {
            assert_eq!(rng.next_u64(), *expected_value, "output #{index} differs from reference");
        }
    }

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut a = RetreatRng::from_seed(7);
        let mut b = RetreatRng::from_seed(7);
        for _ in 0..SAMPLE_COUNT {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_give_different_sequences() {
        let mut a = RetreatRng::from_seed(1);
        let mut b = RetreatRng::from_seed(2);
        let a_values: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let b_values: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_ne!(a_values, b_values);
    }

    #[test]
    fn next_f32_stays_in_zero_to_one() {
        let mut rng = RetreatRng::from_seed(3);
        for _ in 0..SAMPLE_COUNT {
            let value = rng.next_f32();
            assert!((0.0..1.0).contains(&value), "next_f32 returned {value}, outside [0, 1)");
        }
    }

    #[test]
    fn range_f32_stays_in_range() {
        let mut rng = RetreatRng::from_seed(4);
        for _ in 0..SAMPLE_COUNT {
            let value = rng.range_f32(-5.0, 5.0);
            assert!((-5.0..5.0).contains(&value), "range_f32 returned {value}, outside [-5, 5)");
        }
    }

    #[test]
    fn range_f32_with_equal_bounds_returns_that_bound() {
        let mut rng = RetreatRng::from_seed(5);
        assert_eq!(rng.range_f32(2.5, 2.5), 2.5);
    }

    /// The largest value `next_f32` can return: `(2^24 - 1) / 2^24`.
    const LARGEST_UNIT: f32 = 1.0 - 1.0 / (1u32 << F32_RANDOM_BITS) as f32;

    #[test]
    fn scaling_never_rounds_up_to_max() {
        // Each pair rounds up to `max` without the guard (checked with f32 arithmetic).
        for (min, max) in [(100.0, 101.0), (0.1, 0.3), (1.0e6, 1.0e6 + 1.0)] {
            let value = scale_unit_to_half_open_range(LARGEST_UNIT, min, max);
            assert!(value < max, "[{min}, {max}) produced {value}, which is not below max");
            assert!(value >= min, "[{min}, {max}) produced {value}, which is below min");
        }
    }

    #[test]
    fn scaling_zero_returns_min() {
        assert_eq!(scale_unit_to_half_open_range(0.0, -3.0, 7.0), -3.0);
    }

    #[test]
    #[should_panic(expected = "invalid range")]
    fn range_f32_panics_when_min_is_greater_than_max() {
        let mut rng = RetreatRng::from_seed(6);
        rng.range_f32(1.0, 0.0);
    }
}
