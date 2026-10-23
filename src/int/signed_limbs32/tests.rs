use crate::int::limb_layer::tests::limb_layer_tests;

// The tests that every instance shares, with their helpers and the imports that the tests below
// use as well.
limb_layer_tests!(signed unsigned: limbs32);

/// Values around zero, the limb boundaries, the powers of ten and the limits of `i128`.
fn samples() -> Vec<i128> {
  let mut values = vec![
    0,
    1,
    -1,
    9,
    -9,
    i128::MAX,
    i128::MIN,
    i128::MAX - 1,
    i128::MIN + 1,
  ];
  for boundary in [1i128 << 31, 1 << 32, 1 << 63, 1 << 64, 1 << 95, 1 << 96] {
    values.extend([boundary - 1, boundary, boundary + 1]);
    values.extend([1 - boundary, -boundary, -boundary - 1]);
  }
  for exp in 1..=38 {
    let power = 10i128.pow(exp);
    values.extend([power - 1, power, power + 1, 1 - power, -power, -power - 1]);
  }
  values
}

/// The samples that `T` holds, or under Miri every third of them.
fn samples_of<T: TryFrom<i128>>() -> Vec<T> {
  bigint::thin(
    samples()
      .into_iter()
      .filter_map(|value| T::try_from(value).ok())
      .collect(),
  )
}

/// `value` in `N` limbs of two's complement, least significant first. Requires `N <= 4` and
/// `value` to be within the range of `32 * N` bits.
fn limbs_of<T: Into<i128>, const N: usize>(value: T) -> [u32; N] {
  let value = value.into();
  core::array::from_fn(|i| (value >> (32 * i)) as u32)
}

/// The unsigned integer `value` in `N` limbs, least significant first. Requires `N <= 4` and
/// `value < 2^(32 * N)`.
fn unsigned_limbs_of<T: Into<u128>, const N: usize>(value: T) -> [u32; N] {
  let value = value.into();
  core::array::from_fn(|i| (value >> (32 * i)) as u32)
}

/// Defines a test that compares `N` limbs with the native signed integer of `32 * N` bits.
macro_rules! native_oracle {
  ($name:ident, $native:ident, $n:literal) => {
    #[test]
    fn $name() {
      let samples: Vec<$native> = samples_of();
      let max_exp = $native::MAX.ilog10();
      for &a in &samples {
        let limbs_a: [u32; $n] = limbs_of(a);
        assert_eq!(is_negative(&limbs_a), a < 0);
        assert_eq!(is_positive(&limbs_a), a > 0);
        assert_eq!(is_zero(&limbs_a), a == 0);
        assert_eq!(unsigned_abs(&limbs_a), unsigned_limbs_of(a.unsigned_abs()));
        if a != $native::MIN {
          assert_eq!(neg(&limbs_a), limbs_of(-a));
          assert_eq!(abs(&limbs_a), limbs_of(a.abs()));
        }
        for &b in &samples {
          let limbs_b: [u32; $n] = limbs_of(b);
          assert_eq!(cmp(&limbs_a, &limbs_b), a.cmp(&b));
          assert_eq!(le(&limbs_a, &limbs_b), a <= b);
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
          // As for `/` and `%`, a zero divisor and `MIN / -1` are excluded.
          if b != 0 && (a, b) != ($native::MIN, -1) {
            assert_eq!(
              div_rem(&limbs_a, &limbs_b),
              (limbs_of(a / b), limbs_of(a % b)),
              "{a} / {b}"
            );
          }
        }
        for exp in 0..=max_exp {
          let power = $native::pow(10, exp);
          assert_eq!(
            checked_mul_pow10(&limbs_a, exp),
            a.checked_mul(power).map(limbs_of)
          );
          if let Some(product) = a.checked_mul(power) {
            assert_eq!(mul_pow10(&limbs_a, exp), limbs_of(product));
          }
          // As with `/` and `%`, the quotient and the remainder have the sign of `a`.
          assert_eq!(
            div_rem_pow10(&limbs_a, exp),
            (limbs_of(a / power), limbs_of(a % power))
          );
        }

        let mut buf = [0xff; DIGIT_BUF_LEN];
        let start = write_digits(&limbs_a, &mut buf);
        assert_eq!(&buf[start..], a.unsigned_abs().to_string().as_bytes());
        if a >= 0 {
          let digits = a.to_string();
          assert_eq!(
            push_digits(&[0; $n], digits.as_bytes(), 0, digits.len()),
            limbs_a
          );
        }
      }
    }
  };
}

native_oracle!(one_limb_matches_i32, i32, 1);
native_oracle!(two_limbs_match_i64, i64, 2);
native_oracle!(four_limbs_match_i128, i128, 4);
