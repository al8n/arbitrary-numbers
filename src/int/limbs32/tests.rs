use crate::int::{limb_layer::tests::limb_layer_tests, limbs};

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(unsigned);

/// Values around zero, the limb boundaries, the powers of ten and the limit of `u128`.
fn samples() -> Vec<u128> {
  let mut values = vec![0, 1, 2, 9, 10, u128::MAX, u128::MAX - 1];
  for boundary in [1u128 << 32, 1 << 64, 1 << 96, 1 << 127] {
    values.extend([boundary - 1, boundary, boundary + 1]);
  }
  for exp in 1..=38 {
    let power = 10u128.pow(exp);
    values.extend([power - 1, power, power + 1]);
  }
  values
}

/// The samples that `T` holds, or under Miri every third of them.
fn samples_of<T: TryFrom<u128>>() -> Vec<T> {
  bigint::thin(
    samples()
      .into_iter()
      .filter_map(|value| T::try_from(value).ok())
      .collect(),
  )
}

/// `value` in `N` limbs, least significant first. Requires `N <= 4` and `value < 2^(32 * N)`.
fn limbs_of<T: Into<u128>, const N: usize>(value: T) -> [u32; N] {
  let value = value.into();
  core::array::from_fn(|i| (value >> (32 * i)) as u32)
}

/// Defines a test that compares `N` limbs with the native unsigned integer of `32 * N` bits.
macro_rules! native_oracle {
  ($name:ident, $native:ident, $n:literal) => {
    #[test]
    fn $name() {
      let samples: Vec<$native> = samples_of();
      let max_exp = $native::MAX.ilog10();
      for &a in &samples {
        let limbs_a: [u32; $n] = limbs_of(a);
        assert_eq!(is_zero(&limbs_a), a == 0);
        assert_eq!(is_positive(&limbs_a), a > 0);
        for &b in &samples {
          let limbs_b: [u32; $n] = limbs_of(b);
          assert_eq!(cmp(&limbs_a, &limbs_b), a.cmp(&b));
          assert_eq!(le(&limbs_a, &limbs_b), a <= b);
          let (sum, carry) = a.overflowing_add(b);
          assert_eq!(overflowing_add(&limbs_a, &limbs_b), (limbs_of(sum), carry));
          let (difference, borrow) = a.overflowing_sub(b);
          assert_eq!(
            overflowing_sub(&limbs_a, &limbs_b),
            (limbs_of(difference), borrow)
          );
          assert_eq!(
            checked_add(&limbs_a, &limbs_b),
            a.checked_add(b).map(limbs_of)
          );
          assert_eq!(
            checked_sub(&limbs_a, &limbs_b),
            a.checked_sub(b).map(limbs_of)
          );
          if let Some(product) = a.checked_mul(b) {
            assert_eq!(mul(&limbs_a, &limbs_b), limbs_of(product));
          }
          if b != 0 {
            assert_eq!(
              div_rem(&limbs_a, &limbs_b),
              (limbs_of(a / b), limbs_of(a % b)),
              "{a} / {b}"
            );
          }
        }
        // Up to two exponents beyond the largest power of ten that the type holds.
        for exp in 0..=max_exp + 2 {
          let power = $native::checked_pow(10, exp);
          // Zero times any power is zero; otherwise the power itself must fit.
          let product = match a {
            0 => Some(0),
            _ => power.and_then(|power| a.checked_mul(power)),
          };
          assert_eq!(checked_mul_pow10(&limbs_a, exp), product.map(limbs_of));
          if let Some(product) = product {
            assert_eq!(mul_pow10(&limbs_a, exp), limbs_of(product));
          }
          let (quotient, remainder) = div_rem_pow10(&limbs_a, exp);
          match power {
            Some(power) => assert_eq!(
              (quotient, remainder),
              (limbs_of(a / power), limbs_of(a % power))
            ),
            None => assert_eq!((quotient, remainder), ([0; $n], limbs_a)),
          }
        }

        let mut buf = [0xff; DIGIT_BUF_LEN];
        let start = write_digits(&limbs_a, &mut buf);
        assert_eq!(&buf[start..], a.to_string().as_bytes());
      }
      for exp in 0..=max_exp {
        assert_eq!(pow10::<$n>(exp), limbs_of($native::pow(10, exp)));
        assert_eq!(
          max_magnitude::<$n>(exp),
          limbs_of($native::pow(10, exp) - 1)
        );
      }
    }
  };
}

native_oracle!(one_limb_matches_u32, u32, 1);
native_oracle!(two_limbs_match_u64, u64, 2);
native_oracle!(four_limbs_match_u128, u128, 4);

/// Steps whose window has the top divisor limb on top, where the quotient of the window's top two
/// limbs by that limb is `2^32` or `2^32 + 1`.
#[test]
fn division_estimates_reach_two_pow_32() {
  division_with_equal_top_limbs();
}

/// Joins each pair of limbs into a `u64` limb, the low one first. Requires `M == 2 * N`.
const fn join<const M: usize, const N: usize>(value: &[u32; M]) -> [u64; N] {
  let mut joined = [0; N];
  let mut i = 0;
  while i < N {
    joined[i] = value[2 * i] as u64 | (value[2 * i + 1] as u64) << 32;
    i += 1;
  }
  joined
}

/// Checks the layer for `[u32; M]` against the layer for `[u64; N]`, which holds the same integers
/// in half as many limbs. It allocates nothing, so that it also runs on 32-bit Miri, where the
/// tests against `num-bigint` run out of addresses.
fn matches_the_u64_layer<const M: usize, const N: usize>() {
  const { assert!(M == 2 * N) };
  let mut rng = Rng(0x5eed_3264 ^ N as u64);
  let count = if cfg!(miri) { 16 } else { 2000 };
  for _ in 0..count {
    // Nonzero values, whose limbs have every normalization shift of a `u32`.
    let (a, b) = (rng.limbs::<M>(), rng.limbs::<M>());
    let (wide_a, wide_b): ([u64; N], [u64; N]) = (join(&a), join(&b));
    assert_eq!(cmp(&a, &b), limbs::cmp(&wide_a, &wide_b));
    assert_eq!(le(&a, &b), limbs::le(&wide_a, &wide_b));
    let (sum, carry) = overflowing_add(&a, &b);
    assert_eq!(
      (join(&sum), carry),
      limbs::overflowing_add(&wide_a, &wide_b)
    );
    let (difference, borrow) = overflowing_sub(&a, &b);
    assert_eq!(
      (join(&difference), borrow),
      limbs::overflowing_sub(&wide_a, &wide_b)
    );
    let (quotient, remainder) = div_rem(&a, &b);
    assert_eq!(
      (join(&quotient), join(&remainder)),
      limbs::div_rem(&wide_a, &wide_b),
      "{a:x?} / {b:x?}"
    );
    for mode in bigint::MODES {
      let (rounded, remainder, away) = round_div(&a, &b, mode);
      assert_eq!(
        (join(&rounded), join(&remainder), away),
        limbs::round_div(&wide_a, &wide_b, mode)
      );
    }

    // The low halves, whose product is representable.
    let (mut low_a, mut low_b) = (a, b);
    low_a[N..].fill(0);
    low_b[N..].fill(0);
    let (wide_low_a, wide_low_b): ([u64; N], [u64; N]) = (join(&low_a), join(&low_b));
    assert_eq!(
      join(&mul(&low_a, &low_b)),
      limbs::mul(&wide_low_a, &wide_low_b)
    );

    let exp = (rng.next() % (20 * N as u64 + 2)) as u32;
    let product = checked_mul_pow10(&a, exp);
    assert_eq!(
      product.map(|product| join(&product)),
      limbs::checked_mul_pow10(&wide_a, exp)
    );
    if product.is_some() {
      assert_eq!(join(&mul_pow10(&a, exp)), limbs::mul_pow10(&wide_a, exp));
    }
    let (quotient, remainder) = div_rem_pow10(&a, exp);
    assert_eq!(
      (join(&quotient), join(&remainder)),
      limbs::div_rem_pow10(&wide_a, exp)
    );

    let bits = (rng.next() % (64 * N as u64 + 1)) as u32;
    assert_eq!(fits_bits(&a, bits), limbs::fits_bits(&wide_a, bits));
    assert_eq!(join(&max_of_bits::<M>(bits)), limbs::max_of_bits::<N>(bits));

    // The digits agree and read back. Both layers need `20 * N` bytes at most.
    let (mut buf, mut wide_buf) = ([0; 160], [0; 160]);
    let start = write_digits_into(&a, &mut buf);
    let wide_start = limbs::write_digits_into(&wide_a, &mut wide_buf);
    assert_eq!(&buf[start..], &wide_buf[wide_start..]);
    assert_eq!(push_digits(&[0; M], &buf, start, buf.len()), a);
  }
}

#[test]
fn pairs_of_limbs_match_the_u64_layer() {
  matches_the_u64_layer::<2, 1>();
  matches_the_u64_layer::<4, 2>();
  matches_the_u64_layer::<6, 3>();
  matches_the_u64_layer::<8, 4>();
  matches_the_u64_layer::<16, 8>();
}
