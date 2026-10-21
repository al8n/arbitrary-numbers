use core::cmp::Ordering;
use std::{string::ToString, vec, vec::Vec};

use num_bigint::BigInt;

use super::{
  abs, checked_add, checked_from_unsigned, checked_mul_pow10, checked_resize, checked_sub,
  checked_to_unsigned, cmp, div_rem_pow10, is_negative, is_positive, is_zero, le, mul, mul_pow10,
  neg, push_digits, resize, unsigned_abs, write_digits,
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
  let mut buf = [0; DIGIT_BUF_LEN];
  assert_eq!(
    write_digits(min::<5>(), &mut buf),
    0,
    "|i320::MIN| fills the buffer"
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
