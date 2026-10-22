use core::cmp::Ordering;
use std::{string::ToString, vec, vec::Vec};

use num_bigint::BigInt;

use super::{
  checked_add, checked_mul_pow10, checked_resize, checked_sub, cmp, div_rem_pow10, is_positive,
  is_zero, le, max_magnitude, mul, mul_pow10, overflowing_add, overflowing_sub, pow10, push_digits,
  resize, write_digits,
};
use crate::{bigint, int::DIGIT_BUF_LEN};

fn from_u128(value: u128) -> [u64; 2] {
  [value as u64, (value >> 64) as u64]
}

fn to_u128(limbs: [u64; 2]) -> u128 {
  limbs[0] as u128 | (limbs[1] as u128) << 64
}

/// Values around zero, the limb boundary, the powers of ten and the limit of `u128`.
fn samples() -> Vec<u128> {
  let mut values = vec![0, 1, 2, 9, 10, u128::MAX, u128::MAX - 1];
  for boundary in [1u128 << 64, 1 << 127] {
    values.extend([boundary - 1, boundary, boundary + 1]);
  }
  for exp in 1..=38 {
    let power = 10u128.pow(exp);
    values.extend([power - 1, power, power + 1]);
  }
  values
}

#[test]
fn one_limb_matches_u64() {
  let samples: Vec<u64> = samples()
    .into_iter()
    .filter_map(|value| value.try_into().ok())
    .collect();
  for &a in &samples {
    assert_eq!(is_zero([a]), a == 0);
    assert_eq!(is_positive([a]), a > 0);
    for &b in &samples {
      assert_eq!(cmp([a], [b]), a.cmp(&b));
      assert_eq!(le([a], [b]), a <= b);
      let (sum, carry) = a.overflowing_add(b);
      assert_eq!(overflowing_add([a], [b]), ([sum], carry));
      let (difference, borrow) = a.overflowing_sub(b);
      assert_eq!(overflowing_sub([a], [b]), ([difference], borrow));
      if let Some(product) = a.checked_mul(b) {
        assert_eq!(mul([a], [b]), [product]);
      }
    }
    for exp in 0..=25 {
      let power = 10u128.pow(exp);
      let product = (a as u128)
        .checked_mul(power)
        .and_then(|product| u64::try_from(product).ok());
      assert_eq!(
        checked_mul_pow10([a], exp),
        product.map(|product| [product])
      );
      if let Some(product) = product {
        assert_eq!(mul_pow10([a], exp), [product]);
      }
      let quotient = (a as u128 / power) as u64;
      let remainder = (a as u128 % power) as u64;
      assert_eq!(div_rem_pow10([a], exp), ([quotient], [remainder]));
    }
  }
}

#[test]
fn two_limbs_match_u128() {
  for a in samples() {
    let limbs_a = from_u128(a);
    assert_eq!(is_zero(limbs_a), a == 0);
    for b in samples() {
      let limbs_b = from_u128(b);
      assert_eq!(cmp(limbs_a, limbs_b), a.cmp(&b));
      assert_eq!(le(limbs_a, limbs_b), a <= b);
      let (sum, carry) = a.overflowing_add(b);
      assert_eq!(overflowing_add(limbs_a, limbs_b), (from_u128(sum), carry));
      let (difference, borrow) = a.overflowing_sub(b);
      assert_eq!(
        overflowing_sub(limbs_a, limbs_b),
        (from_u128(difference), borrow)
      );
      assert_eq!(
        checked_add(limbs_a, limbs_b),
        a.checked_add(b).map(from_u128)
      );
      assert_eq!(
        checked_sub(limbs_a, limbs_b),
        a.checked_sub(b).map(from_u128)
      );
      if let Some(product) = a.checked_mul(b) {
        assert_eq!(mul(limbs_a, limbs_b), from_u128(product));
      }
    }
    for exp in 0..=40 {
      // Zero times any power is zero; otherwise the power itself must fit.
      let product = match a {
        0 => Some(0),
        _ => 10u128
          .checked_pow(exp)
          .and_then(|power| a.checked_mul(power)),
      };
      assert_eq!(checked_mul_pow10(limbs_a, exp), product.map(from_u128));
      if let Some(product) = product {
        assert_eq!(mul_pow10(limbs_a, exp), from_u128(product));
      }
      let (quotient, remainder) = div_rem_pow10(limbs_a, exp);
      match 10u128.checked_pow(exp) {
        Some(power) => {
          assert_eq!(to_u128(quotient), a / power);
          assert_eq!(to_u128(remainder), a % power);
        }
        None => assert_eq!((quotient, remainder), ([0; 2], limbs_a)),
      }
    }

    let mut buf = [0xff; DIGIT_BUF_LEN];
    let start = write_digits(limbs_a, &mut buf);
    assert_eq!(&buf[start..], a.to_string().as_bytes());
  }
  for exp in 0..=38 {
    assert_eq!(to_u128(pow10(exp)), 10u128.pow(exp));
    assert_eq!(to_u128(max_magnitude(exp)), 10u128.pow(exp) - 1);
  }
}

/// `2^(64k)`.
fn boundary<const N: usize>(k: usize) -> [u64; N] {
  let mut value = [0; N];
  value[k] = 1;
  value
}

/// `2^(64k) - 1`: the `k` lowest limbs all ones.
fn ones_below<const N: usize>(k: usize) -> [u64; N] {
  let mut value = [0; N];
  value[..k].fill(u64::MAX);
  value
}

fn carries_and_borrows<const N: usize>() {
  let one = boundary::<N>(0);
  for k in 1..N {
    // A carry and a borrow across the `k` lowest limbs.
    assert_eq!(checked_add(ones_below::<N>(k), one), Some(boundary::<N>(k)));
    assert_eq!(checked_sub(boundary::<N>(k), one), Some(ones_below::<N>(k)));
    // And across one limb, from limb `k - 1` into limb `k`.
    let mut low = [0; N];
    low[k - 1] = u64::MAX;
    let mut high = [0; N];
    high[k - 1] = 1 << 63;
    let mut carried = [0; N];
    carried[k - 1] = (1 << 63) - 1;
    carried[k] = 1;
    assert_eq!(checked_add(low, high), Some(carried));
    assert_eq!(checked_sub(carried, high), Some(low));
  }
  // Out of the top limb.
  assert_eq!(overflowing_add([u64::MAX; N], one), ([0; N], true));
  assert_eq!(checked_add([u64::MAX; N], one), None);
  assert_eq!(checked_add(boundary::<N>(N - 1), [u64::MAX; N]), None);
  assert_eq!(overflowing_sub([0; N], one), ([u64::MAX; N], true));
  assert_eq!(checked_sub([0; N], one), None);
  assert_eq!(
    checked_sub(ones_below::<N>(N - 1), boundary::<N>(N - 1)),
    None
  );
  assert_eq!(checked_sub([u64::MAX; N], [u64::MAX; N]), Some([0; N]));
}

#[test]
fn carries_and_borrows_cross_every_limb() {
  carries_and_borrows::<3>();
  carries_and_borrows::<4>();
  carries_and_borrows::<5>();
  carries_and_borrows::<6>();
  carries_and_borrows::<7>();
  carries_and_borrows::<8>();
}

fn numeric_order<const N: usize>() {
  // The array order compares limb 0 first and would put 1 above `2^64`.
  let one = boundary::<N>(0);
  let two_pow_64 = boundary::<N>(1);
  assert!(one > two_pow_64);
  assert_eq!(cmp(one, two_pow_64), Ordering::Less);
  assert!(le(one, two_pow_64) && !le(two_pow_64, one));
  assert_eq!(
    cmp(ones_below::<N>(N - 1), boundary::<N>(N - 1)),
    Ordering::Less
  );
  assert_eq!(cmp([u64::MAX; N], [u64::MAX; N]), Ordering::Equal);
}

#[test]
fn comparison_is_numeric() {
  numeric_order::<3>();
  numeric_order::<4>();
  numeric_order::<5>();
  numeric_order::<6>();
  numeric_order::<7>();
  numeric_order::<8>();
}

fn maximal_products<const N: usize>() {
  let max = bigint::from_limbs(&[u64::MAX; N], false);
  let root = max.sqrt();
  let operands = [
    BigInt::ZERO,
    BigInt::from(1u8),
    root.clone(),
    &root + 1u8,
    max.clone(),
    BigInt::from(u64::MAX),
    bigint::from_limbs(&ones_below::<N>(N - 1), false),
    bigint::from_limbs(&boundary::<N>(N - 1), false),
  ];
  for a in &operands {
    for b in &operands {
      let product = a * b;
      let (Some(limbs_a), Some(limbs_b)) =
        (bigint::to_limbs::<N>(a, false), bigint::to_limbs(b, false))
      else {
        panic!("operands fit");
      };
      match bigint::to_limbs::<N>(&product, false) {
        Some(expected) => assert_eq!(mul(limbs_a, limbs_b), expected, "{a} * {b}"),
        // The maximal operands whose product fits are around the square root.
        None => assert!(a > &root || b > &root),
      }
    }
  }
  // The largest square is `root^2`.
  let square = bigint::to_limbs::<N>(&(&root * &root), false).expect("fits");
  let root_limbs = bigint::to_limbs::<N>(&root, false).expect("fits");
  assert_eq!(mul(root_limbs, root_limbs), square);
  assert!(bigint::to_limbs::<N>(&((&root + 1u8) * (&root + 1u8)), false).is_none());
}

#[test]
fn multiplication_with_maximal_operands() {
  maximal_products::<3>();
  maximal_products::<4>();
  maximal_products::<5>();
  maximal_products::<6>();
  maximal_products::<7>();
  maximal_products::<8>();
}

fn division_by_every_power<const N: usize>() {
  let max = bigint::from_limbs(&[u64::MAX; N], false);
  for value in [
    max.clone(),
    bigint::pow10(bigint::max_exp(&max)),
    BigInt::from(1u8),
    BigInt::ZERO,
  ] {
    let limbs = bigint::to_limbs::<N>(&value, false).expect("fits");
    // Beyond the largest power of ten that fits, the quotient is zero.
    for exp in 0..=bigint::max_exp(&max) + 2 {
      let power = bigint::pow10(exp);
      let (quotient, remainder) = div_rem_pow10(limbs, exp);
      assert_eq!(bigint::from_limbs(&quotient, false), &value / &power);
      assert_eq!(bigint::from_limbs(&remainder, false), &value % &power);
    }
  }
}

#[test]
fn div_rem_pow10_for_every_exponent() {
  division_by_every_power::<3>();
  division_by_every_power::<4>();
  division_by_every_power::<5>();
  division_by_every_power::<6>();
  division_by_every_power::<7>();
  division_by_every_power::<8>();
}

fn digits_of_the_limits<const N: usize>() {
  for value in [[0; N], [u64::MAX; N], boundary::<N>(N - 1)] {
    let mut buf = [0xff; DIGIT_BUF_LEN];
    let start = write_digits(value, &mut buf);
    let expected = bigint::from_limbs(&value, false).to_string();
    assert_eq!(&buf[start..], expected.as_bytes());

    // The digits read back, also with separators among them.
    let digits = expected.as_bytes();
    assert_eq!(push_digits([0; N], digits, 0, digits.len()), value);
    let separated: Vec<u8> = digits
      .iter()
      .flat_map(|&digit| [b'_', digit, b'.'])
      .collect();
    assert_eq!(push_digits([0; N], &separated, 0, separated.len()), value);
  }
}

#[test]
fn digits_of_zero_and_max() {
  digits_of_the_limits::<3>();
  digits_of_the_limits::<4>();
  digits_of_the_limits::<5>();
  digits_of_the_limits::<6>();
  digits_of_the_limits::<7>();
  digits_of_the_limits::<8>();
  let mut buf = [0; DIGIT_BUF_LEN];
  assert_eq!(
    write_digits([u64::MAX; 8], &mut buf),
    0,
    "u512::MAX fills the buffer"
  );
}

#[test]
fn resizing() {
  let value = [1, 2, 3];
  assert_eq!(resize::<3, 5>(value), [1, 2, 3, 0, 0]);
  assert_eq!(checked_resize::<3, 5>(value), Some([1, 2, 3, 0, 0]));
  assert_eq!(resize::<3, 2>(value), [1, 2]);
  assert_eq!(checked_resize::<3, 2>(value), None);
  assert_eq!(checked_resize::<3, 2>([1, 2, 0]), Some([1, 2]));
  assert_eq!(checked_resize::<5, 3>([1, 2, 3, 0, 0]), Some(value));
  assert_eq!(checked_resize::<5, 3>([1, 2, 3, 0, 1]), None);
  assert_eq!(checked_resize::<3, 3>([u64::MAX; 3]), Some([u64::MAX; 3]));
  // Unsigned limbs extend with zeros, whatever the top bit.
  assert_eq!(resize::<2, 3>([0, 1 << 63]), [0, 1 << 63, 0]);
}
