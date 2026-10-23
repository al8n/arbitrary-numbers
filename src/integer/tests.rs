use core::{
  fmt::{Debug, Display},
  hash::{BuildHasher, Hash, Hasher},
};
use std::{collections::hash_map::RandomState, format, string::ToString, vec, vec::Vec};

use ::proptest::{prelude::*, test_runner::RngSeed};
use num_bigint::{BigInt, Sign};
use typenum::{
  U1, U2, U7, U63, U64, U65, U100, U127, U128, U129, U160, U200, U255, U256, U257, U511, U1023,
  U4294967295,
};

use crate::{
  Int, Uint, bigint, i192, i256, i320, i384, i448, i512, i768, i1024, i2048, i4096,
  int::DIGIT_BUF_LEN, u192, u256, u320, u384, u448, u512, u768, u1024, u2048, u4096,
};

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
  fn assert_traits<
    T: Copy + Eq + Ord + Hash + Debug + Display + Default + Send + Sync + Unpin + 'static,
  >() {
  }

  assert_traits::<u192>();
  assert_traits::<u256>();
  assert_traits::<u320>();
  assert_traits::<u384>();
  assert_traits::<u448>();
  assert_traits::<u512>();
  assert_traits::<u768>();
  assert_traits::<u1024>();
  assert_traits::<u2048>();
  assert_traits::<u4096>();
  assert_traits::<i192>();
  assert_traits::<i256>();
  assert_traits::<i320>();
  assert_traits::<i384>();
  assert_traits::<i448>();
  assert_traits::<i512>();
  assert_traits::<i768>();
  assert_traits::<i1024>();
  assert_traits::<i2048>();
  assert_traits::<i4096>();
  // Widths that leave bits or limbs unused.
  assert_traits::<Uint<U200, [u64; 4]>>();
  assert_traits::<Uint<U64, [u64; 2]>>();
  assert_traits::<Int<U7, [u64; 1]>>();
  assert_traits::<Int<U100, [u64; 3]>>();

  assert_eq!(size_of::<u192>(), 24);
  assert_eq!(size_of::<i256>(), 32);
  assert_eq!(size_of::<u320>(), 40);
  assert_eq!(size_of::<i384>(), 48);
  assert_eq!(size_of::<u448>(), 56);
  assert_eq!(size_of::<i512>(), 64);
  assert_eq!(size_of::<u768>(), 96);
  assert_eq!(size_of::<i768>(), 96);
  assert_eq!(size_of::<u1024>(), 128);
  assert_eq!(size_of::<i1024>(), 128);
  assert_eq!(size_of::<u2048>(), 256);
  assert_eq!(size_of::<i2048>(), 256);
  assert_eq!(size_of::<u4096>(), 512);
  assert_eq!(size_of::<i4096>(), 512);
  assert_eq!(size_of::<Uint<U200, [u64; 4]>>(), 32);
  assert_eq!(size_of::<Int<U100, [u64; 3]>>(), 24);
  // The alignment of `u64`, 8 bytes on 64-bit targets.
  assert_eq!(align_of::<i320>(), align_of::<u64>());
  assert_eq!(align_of::<u512>(), align_of::<u64>());
  assert_eq!(align_of::<u4096>(), align_of::<u64>());
  assert_eq!(align_of::<i4096>(), align_of::<u64>());
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
            "{:160}",
            "{:^161}",
            "{:+0160}",
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
  unsigned_384: u384, false, 6;
  unsigned_448: u448, false, 7;
  unsigned_512: u512, false, 8;
  signed_192: i192, true, 3;
  signed_256: i256, true, 4;
  signed_320: i320, true, 5;
  signed_384: i384, true, 6;
  signed_448: i448, true, 7;
  signed_512: i512, true, 8;
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
fn the_widest_values_fill_the_digit_buffer() {
  // `u512::MAX` has 155 digits, and `i512::MIN` has 154 and a sign.
  assert_eq!(
    u512::from_limbs([u64::MAX; 8]).to_string().len(),
    DIGIT_BUF_LEN
  );
  assert_eq!(
    i512::from_limbs([0, 0, 0, 0, 0, 0, 0, 1 << 63])
      .to_string()
      .len(),
    DIGIT_BUF_LEN
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

  const MAX: Uint<U200, [u64; 4]> = Uint::<U200, [u64; 4]>::MAX;
  const MIN: Int<U100, [u64; 2]> = Int::<U100, [u64; 2]>::MIN;
  const WIDEST: u4096 = u4096::from_limbs([u64::MAX; 64]);
  const BITS: u32 = i4096::BITS;
  const ACCEPTED: Option<Uint<U65, [u64; 2]>> = Uint::<U65, [u64; 2]>::checked_from_limbs([1, 1]);
  const REJECTED: Option<Int<U7, [u64; 1]>> = Int::<U7, [u64; 1]>::checked_from_limbs([127]);
  const QUERIES: [bool; 3] = [MIN.is_negative(), MAX.is_zero(), MIN.is_positive()];
  const MAX_LIMBS: [u64; 4] = *MAX.as_limbs();

  assert_eq!(MAX_LIMBS, [u64::MAX, u64::MAX, u64::MAX, 0xff]);
  assert_eq!(MIN.to_string(), format!("-{}", BigInt::from(1u8) << 99u32));
  assert_eq!(WIDEST, u4096::MAX);
  assert_eq!(BITS, 4096);
  assert_eq!(
    ACCEPTED.map(|value| value.to_limbs()),
    Some([1, 1]),
    "2^64 + 1 has 65 bits"
  );
  assert_eq!(REJECTED, None, "127 needs 8 bits");
  assert_eq!(QUERIES, [true, false, false]);
}

#[test]
fn the_widest_aliases_format_every_digit() {
  // `2^4096 - 1` has 1234 digits, and `-2^4095` has 1233 and a sign.
  let max = u4096::MAX.to_string();
  assert_eq!(max, bigint::max(4096, false).to_string());
  assert_eq!(max.len(), 1234);
  let min = i4096::MIN.to_string();
  assert_eq!(min, bigint::min(4096, true).to_string());
  assert_eq!(min.len(), 1234);
  assert_eq!(
    format!("{:+}", i4096::MAX),
    format!("+{}", bigint::max(4096, true))
  );
}

#[test]
fn the_largest_width_is_u32_max() {
  // `2^32 - 1` bits in `2^26` limbs. Only constants of the type are used, so no value of half a
  // gibibyte is made.
  assert_eq!(Uint::<U4294967295, [u64; 67108864]>::BITS, u32::MAX);
  assert_eq!(Int::<U4294967295, [u64; 67108864]>::BITS, u32::MAX);
}

/// The proptest configuration: the default, or under Miri a few cases from a fixed seed, written
/// to no file.
fn config() -> ProptestConfig {
  if cfg!(miri) {
    ProptestConfig {
      cases: 4,
      failure_persistence: None,
      rng_seed: RngSeed::Fixed(0x5eed),
      ..ProptestConfig::default()
    }
  } else {
    ProptestConfig::default()
  }
}

/// Tests of one width against `num-bigint`. Each type is listed with its sign, `signed` or
/// `unsigned`, its numbers of bits and limbs, and `full` when the bits fill the limbs.
macro_rules! width_tests {
  ($(
    $module:ident: $ty:ty, $sign:ident, $bits:literal, $limbs:literal $(, $full:ident)?
  );+ $(;)?) => {$(
    mod $module {
      use super::*;

      type T = $ty;

      const SIGNED: bool = width_tests!(@signed $sign);
      const BITS: u32 = $bits;
      const LIMBS: usize = $limbs;

      fn big(value: T) -> BigInt {
        bigint::from_limbs(&value.to_limbs(), SIGNED)
      }

      fn min() -> BigInt {
        bigint::min(BITS, SIGNED)
      }

      fn max() -> BigInt {
        bigint::max(BITS, SIGNED)
      }

      /// The limbs of `value`, in `LIMBS` limbs of two's complement for a signed type.
      fn limbs_of(value: &BigInt) -> [u64; LIMBS] {
        bigint::to_limbs(value, SIGNED).expect("the limbs hold the value")
      }

      /// The values at and next to zero, the limits, the limb boundaries and the sign bit.
      fn values() -> Vec<T> {
        let (min, max) = (min(), max());
        let one = BigInt::from(1u8);
        let mut candidates = vec![
          BigInt::ZERO,
          one.clone(),
          BigInt::from(2u8),
          max.clone(),
          &max - 1u8,
          min.clone(),
          &min + 1u8,
        ];
        for exp in [1, 62, 63, 64, 65, 127, 128, BITS.saturating_sub(2), BITS - 1] {
          let power = &one << exp;
          candidates.extend([&power - 1u8, power]);
        }
        let mut values: Vec<BigInt> = Vec::new();
        for candidate in candidates {
          for value in [-&candidate, candidate] {
            if min <= value && value <= max && !values.contains(&value) {
              values.push(value);
            }
          }
        }
        bigint::thin(values)
          .iter()
          .map(|value| T::checked_from_limbs(limbs_of(value)).expect("in range"))
          .collect()
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn constants_match_big_integers() {
        assert_eq!(T::BITS, BITS);
        assert_eq!(big(T::ZERO), BigInt::ZERO);
        assert_eq!(big(T::MIN), min());
        assert_eq!(big(T::MAX), max());
        // The limits hold their values in every limb, as `checked_from_limbs` requires.
        assert_eq!(T::MIN.to_limbs(), limbs_of(&min()));
        assert_eq!(T::MAX.to_limbs(), limbs_of(&max()));
        assert_eq!(T::checked_from_limbs(T::MIN.to_limbs()), Some(T::MIN));
        assert_eq!(T::checked_from_limbs(T::MAX.to_limbs()), Some(T::MAX));
        assert_eq!(T::default(), T::ZERO);
        assert_eq!(T::ZERO.to_limbs(), [0; LIMBS]);
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn checked_from_limbs_accepts_exactly_the_values() {
        let (min, max) = (min(), max());
        for pattern in bigint::bit_patterns::<LIMBS>(&[BITS]) {
          let value = bigint::from_limbs(&pattern, SIGNED);
          let checked = T::checked_from_limbs(pattern);
          assert_eq!(checked.is_some(), min <= value && value <= max, "{pattern:x?}");
          if let Some(checked) = checked {
            assert_eq!(checked.to_limbs(), pattern);
            assert_eq!(checked.as_limbs(), &pattern);
          }
        }
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn equality_order_and_hash_are_numeric() {
        let values: Vec<(T, BigInt)> = values().into_iter().map(|value| (value, big(value))).collect();
        let state = RandomState::new();
        for &(a, ref big_a) in &values {
          // One `write_u64` for each of the lowest `ceil(Bits / 64)` limbs, least significant
          // first, and no length prefix.
          let expected: Vec<Call> = a.to_limbs()[..BITS.div_ceil(64) as usize]
            .iter()
            .map(|&limb| Call::U64(limb))
            .collect();
          assert_eq!(recorded(&a), expected, "{a}");
          for &(b, ref big_b) in &values {
            assert_eq!(a == b, big_a == big_b);
            assert_eq!(a.cmp(&b), big_a.cmp(big_b), "{a} and {b}");
            assert_eq!(a.partial_cmp(&b), Some(big_a.cmp(big_b)));
            if a == b {
              assert_eq!(state.hash_one(a), state.hash_one(b));
            }
          }
        }
        let mut sorted: Vec<T> = values.iter().map(|&(value, _)| value).collect();
        sorted.sort();
        assert!(sorted.windows(2).all(|pair| big(pair[0]) < big(pair[1])));
      }

      #[test]
      #[cfg_attr(
        all(miri, target_pointer_width = "32"),
        ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
      )]
      fn formatting_matches_big_integers() {
        for value in values() {
          let reference = big(value);
          assert_eq!(value.to_string(), reference.to_string());
          // Under Miri, which divides many limbs slowly, the widest values skip the other formats.
          if LIMBS <= 8 || !cfg!(miri) {
            assert_eq!(format!("{value:?}"), reference.to_string());
            assert_same_format!(
              value,
              reference,
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
              "{:>120?}",
              "{:+0120?}",
            );
          }
        }
      }

      width_tests!(@sign_tests $sign);

      $(width_tests!(@$full);)?

      proptest! {
        #![proptest_config(config())]

        #[test]
        #[cfg_attr(
          all(miri, target_pointer_width = "32"),
          ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
        )]
        fn checked_from_limbs_matches_big_integers(limbs in any::<[u64; LIMBS]>()) {
          let value = bigint::from_limbs(&limbs, SIGNED);
          prop_assert_eq!(T::checked_from_limbs(limbs).is_some(), min() <= value && value <= max());
        }

        // Random limbs rarely hold a value of a width that leaves bits unused, so these hold a
        // value of a random width, which `T` holds when the width is at most `Bits`.
        #[test]
        #[cfg_attr(
          all(miri, target_pointer_width = "32"),
          ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
        )]
        fn checked_from_limbs_matches_big_integers_near_the_limits(
          limbs in any::<[u64; LIMBS]>(),
          width in 1..=64 * LIMBS as u32,
        ) {
          let modulus = BigInt::from(1u8) << width;
          let low = bigint::from_limbs(&limbs, false) % &modulus;
          let value = if SIGNED && low >= &modulus >> 1u8 { low - &modulus } else { low };
          let checked = T::checked_from_limbs(limbs_of(&value));
          prop_assert_eq!(checked.is_some(), min() <= value && value <= max());
          if let Some(checked) = checked {
            prop_assert_eq!(big(checked), value);
          }
        }
      }
    }
  )+};
  (@signed signed) => {
    true
  };
  (@signed unsigned) => {
    false
  };
  (@sign_tests signed) => {
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn sign_queries_match_big_integers() {
      for value in values() {
        let sign = big(value).sign();
        assert_eq!(value.is_zero(), sign == Sign::NoSign, "{value}");
        assert_eq!(value.is_negative(), sign == Sign::Minus, "{value}");
        assert_eq!(value.is_positive(), sign == Sign::Plus, "{value}");
      }
      assert!(T::MIN.is_negative() && !T::MAX.is_negative());
    }
  };
  (@sign_tests unsigned) => {
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn zero_query_matches_big_integers() {
      for value in values() {
        assert_eq!(value.is_zero(), big(value).sign() == Sign::NoSign, "{value}");
      }
    }
  };
  (@full) => {
    #[test]
    #[cfg_attr(
      all(miri, target_pointer_width = "32"),
      ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
    )]
    fn from_limbs_takes_every_pattern() {
      for pattern in bigint::bit_patterns::<LIMBS>(&[BITS]) {
        let value = T::from_limbs(pattern);
        assert_eq!(value.to_limbs(), pattern);
        assert_eq!(T::checked_from_limbs(pattern), Some(value));
      }
    }
  };
}

width_tests!(
  // Widths that fill their limbs: every alias, and one and two limbs.
  uint_64_in_1: Uint<U64, [u64; 1]>, unsigned, 64, 1, full;
  uint_128_in_2: Uint<U128, [u64; 2]>, unsigned, 128, 2, full;
  uint_192: u192, unsigned, 192, 3, full;
  uint_256: u256, unsigned, 256, 4, full;
  uint_320: u320, unsigned, 320, 5, full;
  uint_384: u384, unsigned, 384, 6, full;
  uint_448: u448, unsigned, 448, 7, full;
  uint_512: u512, unsigned, 512, 8, full;
  uint_768: u768, unsigned, 768, 12, full;
  uint_1024: u1024, unsigned, 1024, 16, full;
  uint_2048: u2048, unsigned, 2048, 32, full;
  uint_4096: u4096, unsigned, 4096, 64, full;
  int_64_in_1: Int<U64, [u64; 1]>, signed, 64, 1, full;
  int_128_in_2: Int<U128, [u64; 2]>, signed, 128, 2, full;
  int_192: i192, signed, 192, 3, full;
  int_256: i256, signed, 256, 4, full;
  int_320: i320, signed, 320, 5, full;
  int_384: i384, signed, 384, 6, full;
  int_448: i448, signed, 448, 7, full;
  int_512: i512, signed, 512, 8, full;
  int_768: i768, signed, 768, 12, full;
  int_1024: i1024, signed, 1024, 16, full;
  int_2048: i2048, signed, 2048, 32, full;
  int_4096: i4096, signed, 4096, 64, full;
  // Widths that leave bits of the top limb unused.
  uint_1_in_1: Uint<U1, [u64; 1]>, unsigned, 1, 1;
  uint_7_in_1: Uint<U7, [u64; 1]>, unsigned, 7, 1;
  uint_63_in_1: Uint<U63, [u64; 1]>, unsigned, 63, 1;
  uint_65_in_2: Uint<U65, [u64; 2]>, unsigned, 65, 2;
  uint_127_in_2: Uint<U127, [u64; 2]>, unsigned, 127, 2;
  uint_129_in_3: Uint<U129, [u64; 3]>, unsigned, 129, 3;
  uint_160_in_3: Uint<U160, [u64; 3]>, unsigned, 160, 3;
  uint_200_in_4: Uint<U200, [u64; 4]>, unsigned, 200, 4;
  uint_255_in_4: Uint<U255, [u64; 4]>, unsigned, 255, 4;
  uint_257_in_5: Uint<U257, [u64; 5]>, unsigned, 257, 5;
  uint_511_in_8: Uint<U511, [u64; 8]>, unsigned, 511, 8;
  uint_1023_in_16: Uint<U1023, [u64; 16]>, unsigned, 1023, 16;
  int_1_in_1: Int<U1, [u64; 1]>, signed, 1, 1;
  int_2_in_1: Int<U2, [u64; 1]>, signed, 2, 1;
  int_7_in_1: Int<U7, [u64; 1]>, signed, 7, 1;
  int_63_in_1: Int<U63, [u64; 1]>, signed, 63, 1;
  int_65_in_2: Int<U65, [u64; 2]>, signed, 65, 2;
  int_127_in_2: Int<U127, [u64; 2]>, signed, 127, 2;
  int_129_in_3: Int<U129, [u64; 3]>, signed, 129, 3;
  int_160_in_3: Int<U160, [u64; 3]>, signed, 160, 3;
  int_200_in_4: Int<U200, [u64; 4]>, signed, 200, 4;
  int_255_in_4: Int<U255, [u64; 4]>, signed, 255, 4;
  int_257_in_5: Int<U257, [u64; 5]>, signed, 257, 5;
  int_511_in_8: Int<U511, [u64; 8]>, signed, 511, 8;
  int_1023_in_16: Int<U1023, [u64; 16]>, signed, 1023, 16;
  // Widths that leave limbs unused.
  uint_64_in_2: Uint<U64, [u64; 2]>, unsigned, 64, 2;
  uint_100_in_3: Uint<U100, [u64; 3]>, unsigned, 100, 3;
  int_64_in_2: Int<U64, [u64; 2]>, signed, 64, 2;
  int_100_in_3: Int<U100, [u64; 3]>, signed, 100, 3;
);

/// Asserts that `$narrow` and `$wide`, two storages of one width, make the same `Hash` calls for
/// each of `$values`.
macro_rules! assert_same_hash_calls {
  ($signed:literal, $narrow:ty, $wide:ty, $values:expr) => {
    for value in $values {
      let narrow = <$narrow>::checked_from_limbs(bigint::to_limbs(&value, $signed).expect("fits"))
        .expect("in range");
      let wide = <$wide>::checked_from_limbs(bigint::to_limbs(&value, $signed).expect("fits"))
        .expect("in range");
      assert_eq!(recorded(&narrow), recorded(&wide), "{value}");
    }
  };
}

#[test]
#[cfg_attr(
  all(miri, target_pointer_width = "32"),
  ignore = "Miri on 32-bit targets runs out of addresses in num-bigint allocations"
)]
fn hash_calls_do_not_depend_on_the_number_of_limbs() {
  // The limits of `bits` bits, the values next to them, and the values next to zero.
  fn edges(bits: u32, signed: bool) -> Vec<BigInt> {
    let (min, max) = (bigint::min(bits, signed), bigint::max(bits, signed));
    let mut values = vec![BigInt::ZERO, BigInt::from(1u8), &max - 1u8, max];
    if signed {
      values.extend([BigInt::from(-1), &min + 1u8, min]);
    }
    values
  }

  assert_same_hash_calls!(false, Uint<U1, [u64; 1]>, Uint<U1, [u64; 2]>, edges(1, false));
  assert_same_hash_calls!(false, Uint<U64, [u64; 1]>, Uint<U64, [u64; 2]>, edges(64, false));
  assert_same_hash_calls!(false, Uint<U100, [u64; 2]>, Uint<U100, [u64; 3]>, edges(100, false));
  assert_same_hash_calls!(false, u256, Uint<U256, [u64; 5]>, edges(256, false));
  assert_same_hash_calls!(true, Int<U2, [u64; 1]>, Int<U2, [u64; 3]>, edges(2, true));
  assert_same_hash_calls!(true, Int<U7, [u64; 1]>, Int<U7, [u64; 2]>, edges(7, true));
  assert_same_hash_calls!(true, Int<U64, [u64; 1]>, Int<U64, [u64; 2]>, edges(64, true));
  assert_same_hash_calls!(true, Int<U100, [u64; 2]>, Int<U100, [u64; 3]>, edges(100, true));
  assert_same_hash_calls!(true, i256, Int<U256, [u64; 5]>, edges(256, true));

  // A negative value writes the copies of its sign bit that its lowest limbs hold, and nothing
  // from the limbs above them.
  let minus_one = Int::<U100, [u64; 3]>::checked_from_limbs([u64::MAX; 3]).expect("-1");
  assert_eq!(
    recorded(&minus_one),
    [Call::U64(u64::MAX), Call::U64(u64::MAX)]
  );
}
