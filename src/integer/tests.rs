use core::{
  fmt::{Debug, Display},
  hash::{BuildHasher, Hash, Hasher},
};
use std::{collections::hash_map::RandomState, format, string::ToString, vec, vec::Vec};

use num_bigint::BigInt;

use crate::{bigint, i192, i256, i320, u192, u256, u320};

/// Records the calls that a `Hash` implementation makes.
#[derive(Default)]
struct Recorder {
  calls: Vec<Call>,
}

#[derive(Debug, PartialEq)]
enum Call {
  U64(u64),
  Usize(usize),
  Bytes(Vec<u8>),
}

impl Hasher for Recorder {
  fn finish(&self) -> u64 {
    0
  }

  fn write(&mut self, bytes: &[u8]) {
    self.calls.push(Call::Bytes(bytes.to_vec()));
  }

  fn write_u64(&mut self, value: u64) {
    self.calls.push(Call::U64(value));
  }

  fn write_usize(&mut self, value: usize) {
    self.calls.push(Call::Usize(value));
  }
}

/// Asserts that `$value` formats as `$reference` with each format string.
macro_rules! assert_same_format {
  ($value:expr, $reference:expr, $($spec:tt),+ $(,)?) => {$(
    assert_eq!(format!($spec, $value), format!($spec, $reference), "format {}", $spec);
  )+};
}

fn recorded<T: Hash>(value: &T) -> Vec<Call> {
  let mut recorder = Recorder::default();
  value.hash(&mut recorder);
  recorder.calls
}

#[test]
fn trait_surface_and_layout() {
  fn assert_traits<T: Copy + Eq + Ord + Hash + Debug + Display + Send + Sync + Unpin + 'static>() {}

  assert_traits::<u192>();
  assert_traits::<u256>();
  assert_traits::<u320>();
  assert_traits::<i192>();
  assert_traits::<i256>();
  assert_traits::<i320>();

  assert_eq!(size_of::<u192>(), 24);
  assert_eq!(size_of::<i256>(), 32);
  assert_eq!(size_of::<u320>(), 40);
  assert_eq!(align_of::<i320>(), align_of::<u64>());
  assert_eq!(size_of::<Option<u256>>(), size_of::<Option<[u64; 4]>>());
}

macro_rules! integer_tests {
  ($($module:ident: $name:ident, $signed:literal, $limbs:literal);+ $(;)?) => {$(
    mod $module {
      use super::*;

      type T = $name;

      fn big(value: T) -> BigInt {
        bigint::from_limbs(&value.to_limbs(), $signed)
      }

      /// Limb patterns at zero, the limits, the limb boundaries and the sign bit.
      fn patterns() -> Vec<[u64; $limbs]> {
        let mut patterns = vec![[0; $limbs], [u64::MAX; $limbs]];
        for k in 0..$limbs {
          for limb in [1, u64::MAX, 1 << 63, (1 << 63) - 1] {
            let mut pattern = [0; $limbs];
            pattern[k] = limb;
            patterns.push(pattern);
            let mut pattern = [u64::MAX; $limbs];
            pattern[k] = limb;
            patterns.push(pattern);
          }
        }
        bigint::thin(patterns)
      }

      #[test]
      fn limbs_round_trip() {
        assert_eq!(T::ZERO.to_limbs(), [0; $limbs]);
        assert_eq!(T::ZERO, T::from_limbs([0; $limbs]));
        for pattern in patterns() {
          assert_eq!(T::from_limbs(pattern).to_limbs(), pattern);
        }
      }

      #[test]
      fn equality_and_order_are_numeric() {
        let values: Vec<(T, BigInt)> = patterns()
          .into_iter()
          .map(|pattern| (T::from_limbs(pattern), bigint::from_limbs(&pattern, $signed)))
          .collect();
        for &(a, ref big_a) in &values {
          for &(b, ref big_b) in &values {
            assert_eq!(a == b, big_a == big_b);
            assert_eq!(a.cmp(&b), big_a.cmp(big_b), "{a} and {b}");
            assert_eq!(a.partial_cmp(&b), Some(big_a.cmp(big_b)));
          }
        }
        // The array order compares the least significant limb first, so it puts 1 above 2^64.
        let mut one = [0; $limbs];
        one[0] = 1;
        let mut two_pow_64 = [0; $limbs];
        two_pow_64[1] = 1;
        assert!(one > two_pow_64);
        assert!(T::from_limbs(one) < T::from_limbs(two_pow_64));

        let mut sorted: Vec<T> = values.iter().map(|&(value, _)| value).collect();
        sorted.sort();
        assert!(sorted.windows(2).all(|pair| big(pair[0]) <= big(pair[1])));
      }

      #[test]
      fn hash_writes_the_limbs() {
        let state = RandomState::new();
        for pattern in patterns() {
          let value = T::from_limbs(pattern);
          // One `write_u64` per limb, least significant first, and no length prefix.
          let expected: Vec<Call> = pattern.iter().map(|&limb| Call::U64(limb)).collect();
          assert_eq!(recorded(&value), expected);
          assert_eq!(state.hash_one(value), state.hash_one(T::from_limbs(pattern)));
        }
        // The array's own `Hash` would write a length prefix first.
        assert_eq!(recorded(&[0u64; $limbs])[0], Call::Usize($limbs));
      }

      #[test]
      fn formatting_matches_big_integers() {
        for pattern in patterns() {
          let value = T::from_limbs(pattern);
          let reference = big(value);
          assert_eq!(value.to_string(), reference.to_string());
          assert_eq!(format!("{value:?}"), reference.to_string());
          assert_same_format!(
            value,
            reference,
            "{}",
            "{:1}",
            "{:120}",
            "{:<120}",
            "{:^121}",
            "{:*>120}",
            "{:+}",
            "{:+120}",
            "{:0120}",
            "{:+0120}",
            "{:<0120}",
            "{:?}",
            "{:>120?}",
            "{:+0120?}",
          );
        }
      }
    }
  )+};
}

integer_tests!(
  unsigned_192: u192, false, 3;
  unsigned_256: u256, false, 4;
  unsigned_320: u320, false, 5;
  signed_192: i192, true, 3;
  signed_256: i256, true, 4;
  signed_320: i320, true, 5;
);

#[test]
fn small_values_format_as_native_integers() {
  for value in [0i64, 1, -1, 42, -42, i64::MAX, i64::MIN] {
    let wide = i256::from_limbs(bigint::to_limbs(&BigInt::from(value), true).expect("fits"));
    for (formatted, expected) in [
      (format!("{wide}"), format!("{value}")),
      (format!("{wide:+}"), format!("{value:+}")),
      (format!("{wide:08}"), format!("{value:08}")),
      (format!("{wide:+08}"), format!("{value:+08}")),
      (format!("{wide:_^9}"), format!("{value:_^9}")),
      (format!("{wide:<6?}"), format!("{value:<6?}")),
    ] {
      assert_eq!(formatted, expected);
    }
  }
  for value in [0u64, 7, u64::MAX] {
    let wide = u192::from_limbs([value, 0, 0]);
    assert_eq!(format!("{wide:+05}"), format!("{value:+05}"));
    assert_eq!(format!("{wide:>25}"), format!("{value:>25}"));
  }
}

#[test]
fn signed_order_runs_from_min_to_max() {
  let min = i192::from_limbs([0, 0, 1 << 63]);
  let minus_one = i192::from_limbs([u64::MAX; 3]);
  let one = i192::from_limbs([1, 0, 0]);
  let max = i192::from_limbs([u64::MAX, u64::MAX, (1 << 63) - 1]);
  assert!(min < minus_one && minus_one < i192::ZERO && i192::ZERO < one && one < max);
  // As unsigned limbs, -1 is the largest value.
  assert!(minus_one.to_limbs() > max.to_limbs());
  assert_eq!(min.to_string(), format!("-{}", BigInt::from(1u8) << 191u32));
  assert_eq!(
    max.to_string(),
    ((BigInt::from(1u8) << 191u32) - 1u8).to_string()
  );

  let unsigned_max = u320::from_limbs([u64::MAX; 5]);
  assert!(u320::ZERO < unsigned_max);
  assert_eq!(unsigned_max.to_string().len(), 97);
  assert_eq!(
    i320::from_limbs([0, 0, 0, 0, 1 << 63]).to_string().len(),
    98
  );
}

#[test]
fn constants_in_const_context() {
  const LIMBS: [u64; 4] = [5, 1, 0, 0];
  const VALUE: u256 = u256::from_limbs(LIMBS);
  const BACK: [u64; 4] = VALUE.to_limbs();
  const ZERO: i320 = i320::ZERO;

  assert_eq!(BACK, LIMBS);
  assert_eq!(VALUE.to_string(), "18446744073709551621");
  assert_eq!(ZERO.to_limbs(), [0; 5]);
}
