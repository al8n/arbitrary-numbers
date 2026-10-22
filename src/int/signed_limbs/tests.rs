use core::cmp::Ordering;
use std::{string::ToString, vec, vec::Vec};

use num_bigint::BigInt;

use super::{
  abs, checked_add, checked_from_unsigned, checked_mul_pow10, checked_resize, checked_sub,
  checked_to_unsigned, cmp, div_rem, div_rem_pow10, is_negative, is_positive, is_zero, le, mul,
  mul_pow10, neg, push_digits, resize, round_div, unsigned_abs, write_digits,
};
use crate::{bigint, int::DIGIT_BUF_LEN};

fn from_i128(value: i128) -> [u64; 2] {
  [value as u64, (value >> 64) as u64]
}

fn to_i128(limbs: [u64; 2]) -> i128 {
  (limbs[0] as u128 | (limbs[1] as u128) << 64) as i128
}

/// Values around zero, the limb boundary, the powers of ten and the limits of `i128`.
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
  for boundary in [1i128 << 63, 1 << 64] {
    values.extend([boundary - 1, boundary, boundary + 1]);
    values.extend([1 - boundary, -boundary, -boundary - 1]);
  }
  for exp in 1..=38 {
    let power = 10i128.pow(exp);
    values.extend([power - 1, power, power + 1, 1 - power, -power, -power - 1]);
  }
  values
}

#[test]
fn one_limb_matches_i64() {
  let samples: Vec<i64> = samples()
    .into_iter()
    .filter_map(|value| value.try_into().ok())
    .collect();
  for &a in &samples {
    let limbs_a = [a as u64];
    assert_eq!(is_negative(limbs_a), a < 0);
    assert_eq!(is_positive(limbs_a), a > 0);
    assert_eq!(is_zero(limbs_a), a == 0);
    assert_eq!(unsigned_abs(limbs_a), [a.unsigned_abs()]);
    if a != i64::MIN {
      assert_eq!(neg(limbs_a), [(-a) as u64]);
      assert_eq!(abs(limbs_a), [a.unsigned_abs()]);
    }
    for &b in &samples {
      let limbs_b = [b as u64];
      assert_eq!(cmp(limbs_a, limbs_b), a.cmp(&b));
      assert_eq!(le(limbs_a, limbs_b), a <= b);
      assert_eq!(
        checked_add(limbs_a, limbs_b),
        a.checked_add(b).map(|sum| [sum as u64])
      );
      assert_eq!(
        checked_sub(limbs_a, limbs_b),
        a.checked_sub(b).map(|diff| [diff as u64])
      );
      if let Some(product) = a.checked_mul(b) {
        assert_eq!(mul(limbs_a, limbs_b), [product as u64]);
      }
    }
    for exp in 0..=18 {
      let power = 10i64.pow(exp);
      assert_eq!(
        checked_mul_pow10(limbs_a, exp),
        a.checked_mul(power).map(|product| [product as u64])
      );
      if let Some(product) = a.checked_mul(power) {
        assert_eq!(mul_pow10(limbs_a, exp), [product as u64]);
      }
      // As with `/` and `%`, the quotient and the remainder have the sign of `a`.
      let (quotient, remainder) = div_rem_pow10(limbs_a, exp);
      assert_eq!(
        (quotient[0] as i64, remainder[0] as i64),
        (a / power, a % power)
      );
    }
  }
}

#[test]
fn two_limbs_match_i128() {
  for a in samples() {
    let limbs_a = from_i128(a);
    assert_eq!(is_negative(limbs_a), a < 0);
    assert_eq!(is_positive(limbs_a), a > 0);
    assert_eq!(unsigned_abs(limbs_a), {
      let magnitude = a.unsigned_abs();
      [magnitude as u64, (magnitude >> 64) as u64]
    });
    if a != i128::MIN {
      assert_eq!(neg(limbs_a), from_i128(-a));
      assert_eq!(abs(limbs_a), from_i128(a.abs()));
    }
    for b in samples() {
      let limbs_b = from_i128(b);
      assert_eq!(cmp(limbs_a, limbs_b), a.cmp(&b));
      assert_eq!(le(limbs_a, limbs_b), a <= b);
      assert_eq!(
        checked_add(limbs_a, limbs_b),
        a.checked_add(b).map(from_i128)
      );
      assert_eq!(
        checked_sub(limbs_a, limbs_b),
        a.checked_sub(b).map(from_i128)
      );
      if let Some(product) = a.checked_mul(b) {
        assert_eq!(mul(limbs_a, limbs_b), from_i128(product));
      }
    }
    for exp in 0..=38 {
      let power = 10i128.pow(exp);
      assert_eq!(
        checked_mul_pow10(limbs_a, exp),
        a.checked_mul(power).map(from_i128)
      );
      if let Some(product) = a.checked_mul(power) {
        assert_eq!(mul_pow10(limbs_a, exp), from_i128(product));
      }
      let (quotient, remainder) = div_rem_pow10(limbs_a, exp);
      assert_eq!(
        (to_i128(quotient), to_i128(remainder)),
        (a / power, a % power)
      );
    }

    let mut buf = [0xff; DIGIT_BUF_LEN];
    let start = write_digits(limbs_a, &mut buf);
    assert_eq!(&buf[start..], a.unsigned_abs().to_string().as_bytes());
    if a >= 0 {
      let digits = a.to_string();
      assert_eq!(
        push_digits([0; 2], digits.as_bytes(), 0, digits.len()),
        limbs_a
      );
    }
  }
}

/// `MIN`, with only the sign bit set.
fn min<const N: usize>() -> [u64; N] {
  let mut min = [0; N];
  min[N - 1] = 1 << 63;
  min
}

/// `MAX`, with every bit but the sign bit set.
fn max<const N: usize>() -> [u64; N] {
  let mut max = [u64::MAX; N];
  max[N - 1] = (1 << 63) - 1;
  max
}

fn sign_transitions<const N: usize>() {
  let one = {
    let mut one = [0; N];
    one[0] = 1;
    one
  };
  let minus_one = [u64::MAX; N];
  // The top limb's sign bit flips exactly at `MAX + 1` and `MIN - 1`.
  assert!(!is_negative(max::<N>()) && is_negative(min::<N>()));
  assert_eq!(checked_add(max::<N>(), one), None);
  assert_eq!(checked_sub(min::<N>(), one), None);
  assert_eq!(checked_add(min::<N>(), minus_one), None);
  assert_eq!(checked_sub(max::<N>(), minus_one), None);
  assert_eq!(checked_add(minus_one, one), Some([0; N]));
  assert_eq!(checked_add(max::<N>(), min::<N>()), Some(minus_one));
  assert_eq!(checked_sub(min::<N>(), min::<N>()), Some([0; N]));
  assert_eq!(
    checked_sub([0; N], max::<N>()),
    Some({
      let mut negated = min::<N>();
      negated[0] = 1;
      negated
    })
  );
  assert_eq!(checked_sub([0; N], min::<N>()), None);

  // Numeric order, where the unsigned order of the limbs puts the negative values on top.
  assert_eq!(cmp(min::<N>(), minus_one), Ordering::Less);
  assert_eq!(cmp(minus_one, [0; N]), Ordering::Less);
  assert_eq!(cmp([0; N], one), Ordering::Less);
  assert_eq!(cmp(one, max::<N>()), Ordering::Less);
  assert_eq!(cmp(minus_one, max::<N>()), Ordering::Less);
  assert_eq!(super::limbs::cmp(minus_one, max::<N>()), Ordering::Greater);

  // `MIN` has no negation, but its magnitude is unsigned.
  assert_eq!(unsigned_abs(min::<N>()), min::<N>());
  assert_eq!(neg(max::<N>()), {
    let mut negated = min::<N>();
    negated[0] = 1;
    negated
  });
  assert_eq!(abs(minus_one), one);

  // Scaling reaches the limits on both sides, and one step beyond them fails; the magnitude of
  // `MIN` itself scales by `10^0`.
  assert_eq!(checked_mul_pow10(min::<N>(), 0), Some(min::<N>()));
  assert_eq!(mul_pow10(min::<N>(), 0), min::<N>());
  for limit in [min::<N>(), max::<N>()] {
    let limit = bigint::from_limbs(&limit, true);
    let inside = &limit / 10u8;
    let outside = if limit < BigInt::ZERO {
      &inside - 1u8
    } else {
      &inside + 1u8
    };
    let scaled = bigint::to_limbs::<N>(&(&inside * 10u8), true);
    assert!(scaled.is_some());
    let inside = bigint::to_limbs::<N>(&inside, true).expect("fits");
    let outside = bigint::to_limbs::<N>(&outside, true).expect("fits");
    assert_eq!(checked_mul_pow10(inside, 1), scaled);
    assert_eq!(checked_mul_pow10(outside, 1), None);
  }
}

#[test]
fn sign_transitions_of_the_top_limb() {
  sign_transitions::<3>();
  sign_transitions::<4>();
  sign_transitions::<5>();
  sign_transitions::<6>();
  sign_transitions::<7>();
  sign_transitions::<8>();
}

fn digits_of_min_and_max<const N: usize>() {
  for value in [min::<N>(), max::<N>(), [u64::MAX; N], [0; N]] {
    let mut buf = [0xff; DIGIT_BUF_LEN];
    let start = write_digits(value, &mut buf);
    let expected = bigint::from_limbs(&value, true).magnitude().to_string();
    assert_eq!(&buf[start..], expected.as_bytes());
  }
}

#[test]
fn digits_of_zero_min_and_max() {
  digits_of_min_and_max::<3>();
  digits_of_min_and_max::<4>();
  digits_of_min_and_max::<5>();
  digits_of_min_and_max::<6>();
  digits_of_min_and_max::<7>();
  digits_of_min_and_max::<8>();
  let mut buf = [0; DIGIT_BUF_LEN];
  assert_eq!(
    write_digits(min::<8>(), &mut buf),
    1,
    "|i512::MIN| has one digit fewer than the buffer"
  );
}

/// Every conversion between two and three limbs of either sign, against `num-bigint`.
#[test]
fn conversions_between_signs_and_widths() {
  let mut values: Vec<BigInt> = vec![BigInt::ZERO, BigInt::from(1u8), BigInt::from(-1)];
  for bits in [63u32, 64, 127, 128, 191, 192] {
    let power = BigInt::from(1u8) << bits;
    values.extend([&power - 1u8, power.clone(), &power + 1u8]);
    values.extend([1u8 - &power, -&power, -&power - 1u8]);
  }
  for value in &values {
    let two_signed = bigint::to_limbs::<2>(value, true);
    let two_unsigned = bigint::to_limbs::<2>(value, false);
    let three_signed = bigint::to_limbs::<3>(value, true);
    let three_unsigned = bigint::to_limbs::<3>(value, false);

    if let Some(limbs) = two_signed {
      assert_eq!(checked_resize::<2, 3>(limbs), three_signed);
      assert_eq!(resize::<2, 3>(limbs), three_signed.expect("wider"));
      assert_eq!(checked_to_unsigned::<2, 3>(limbs), three_unsigned);
      assert_eq!(checked_to_unsigned::<2, 2>(limbs), two_unsigned);
    }
    if let Some(limbs) = three_signed {
      assert_eq!(checked_resize::<3, 2>(limbs), two_signed);
      assert_eq!(checked_to_unsigned::<3, 2>(limbs), two_unsigned);
      if let Some(narrow) = two_signed {
        assert_eq!(resize::<3, 2>(limbs), narrow);
      }
    }
    if let Some(limbs) = two_unsigned {
      assert_eq!(checked_from_unsigned::<2, 3>(limbs), three_signed);
      assert_eq!(checked_from_unsigned::<2, 2>(limbs), two_signed);
    }
    if let Some(limbs) = three_unsigned {
      assert_eq!(checked_from_unsigned::<3, 2>(limbs), two_signed);
      assert_eq!(checked_from_unsigned::<3, 3>(limbs), three_signed);
    }
  }
}

#[test]
fn division_matches_i64_and_i128() {
  let samples = bigint::thin(samples());
  for &a in &samples {
    // As for `/` and `%`, a zero divisor and `MIN / -1` are excluded.
    for &b in samples.iter().filter(|&&b| b != 0) {
      if let (Ok(a), Ok(b)) = (i64::try_from(a), i64::try_from(b))
        && (a, b) != (i64::MIN, -1)
      {
        let (quotient, remainder) = div_rem([a as u64], [b as u64]);
        assert_eq!(
          (quotient[0] as i64, remainder[0] as i64),
          (a / b, a % b),
          "{a} / {b}"
        );
      }
      if (a, b) != (i128::MIN, -1) {
        let (quotient, remainder) = div_rem(from_i128(a), from_i128(b));
        assert_eq!(
          (to_i128(quotient), to_i128(remainder)),
          (a / b, a % b),
          "{a} / {b}"
        );
      }
    }
  }
}

/// The value `value`, which must fit `N` limbs.
fn limbs<const N: usize>(value: i64) -> [u64; N] {
  bigint::to_limbs(&BigInt::from(value), true).expect("fits")
}

fn division_with_min<const N: usize>() {
  let min = min::<N>();
  let one = limbs::<N>(1);
  let mut half_min = [0; N];
  half_min[N - 1] = 0xc000_0000_0000_0000;
  assert_eq!(div_rem(min, one), (min, [0; N]));
  assert_eq!(div_rem(min, limbs(2)), (half_min, [0; N]));
  assert_eq!(div_rem(min, min), (one, [0; N]));
  // `2^(64N - 1) = 2 (mod 3)`, an odd power of two, so the remainder is `-2`.
  let (quotient, remainder) = div_rem(min, limbs(3));
  assert_eq!(remainder, limbs(-2));
  assert_eq!(
    bigint::from_limbs(&quotient, true),
    bigint::min(64 * N as u32, true) / 3
  );
  let min_plus_one =
    bigint::to_limbs(&(bigint::min(64 * N as u32, true) + 1u8), true).expect("fits");
  assert_eq!(div_rem(min_plus_one, limbs(-1)), (max::<N>(), [0; N]));
  assert_eq!(div_rem(limbs::<N>(-7), limbs(2)), (limbs(-3), limbs(-1)));
  assert_eq!(div_rem(limbs::<N>(7), limbs(-2)), (limbs(-3), limbs(1)));
  assert_eq!(div_rem(limbs::<N>(-7), limbs(-2)), (limbs(3), limbs(-1)));
}

#[test]
fn division_with_min_for_every_width() {
  division_with_min::<1>();
  division_with_min::<2>();
  division_with_min::<3>();
  division_with_min::<4>();
  division_with_min::<5>();
  division_with_min::<6>();
  division_with_min::<7>();
  division_with_min::<8>();
}

/// A deterministic xorshift generator, so that a failure replays from the seed.
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    self.0 ^= self.0 >> 12;
    self.0 ^= self.0 << 25;
    self.0 ^= self.0 >> 27;
    self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
  }

  /// A value of either sign whose magnitude has a random number of significant limbs.
  fn signed<const N: usize>(&mut self) -> [u64; N] {
    let mut magnitude = [0; N];
    let count = (self.next() % N as u64) as usize + 1;
    for limb in &mut magnitude[..count] {
      *limb = self.next();
    }
    magnitude[count - 1] >>= self.next() % 64;
    magnitude[N - 1] &= u64::MAX >> 1;
    if self.next().is_multiple_of(2) {
      magnitude
    } else {
      neg(magnitude)
    }
  }
}

fn division_against_big_integers<const N: usize>() {
  let mut rng = Rng(0x0dd_ba11 ^ N as u64);
  let count = if cfg!(miri) { 8 } else { 2000 };
  for _ in 0..count {
    let (a, b) = (rng.signed::<N>(), rng.signed::<N>());
    if is_zero(b) {
      continue;
    }
    let (quotient, remainder) = div_rem(a, b);
    let (big_a, big_b) = (bigint::from_limbs(&a, true), bigint::from_limbs(&b, true));
    // `num-bigint` rounds toward zero, as `/` and `%` do.
    assert_eq!(bigint::from_limbs(&quotient, true), &big_a / &big_b);
    assert_eq!(bigint::from_limbs(&remainder, true), &big_a % &big_b);
  }
}

#[test]
fn division_against_big_integers_for_every_width() {
  division_against_big_integers::<3>();
  division_against_big_integers::<4>();
  division_against_big_integers::<8>();
}

/// Checks `round_div(value, divisor, mode)` against rounding by distance.
fn check_rounding<const N: usize>(value: [u64; N], divisor: [u64; N]) {
  let (big_value, big_divisor) = (
    bigint::from_limbs(&value, true),
    bigint::from_limbs(&divisor, true),
  );
  let truncated = &big_value / &big_divisor;
  for mode in bigint::MODES {
    let (rounded, remainder, away) = round_div(value, divisor, mode);
    let rounded = bigint::from_limbs(&rounded, true);
    assert_eq!(
      rounded,
      bigint::round(&big_value, &big_divisor, mode),
      "{big_value} / {big_divisor}, {mode:?}"
    );
    assert_eq!(
      bigint::from_limbs(&remainder, true),
      &big_value % &big_divisor
    );
    assert_eq!(away, rounded != truncated);
  }
}

fn rounding_division<const N: usize>() {
  // Values below the divisor in magnitude, whose quotient is zero, and ties of each parity.
  for value in [-9, -6, -5, -4, -1, 0, 1, 4, 5, 6, 9, -15, -25, 15, 25] {
    check_rounding(limbs::<N>(value), limbs(10));
  }
  check_rounding(min::<N>(), limbs(3));
  check_rounding(max::<N>(), limbs(2));
  let mut rng = Rng(0xfee1_600d ^ N as u64);
  let count = if cfg!(miri) { 4 } else { 600 };
  for _ in 0..count {
    let divisor = unsigned_abs(rng.signed::<N>());
    if is_zero(divisor) {
      continue;
    }
    check_rounding(rng.signed::<N>(), divisor);
  }
}

#[test]
fn rounding_division_matches_rounding_by_distance() {
  rounding_division::<1>();
  rounding_division::<2>();
  rounding_division::<3>();
  rounding_division::<8>();
}
