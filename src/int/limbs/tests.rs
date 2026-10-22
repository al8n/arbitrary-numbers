use core::cmp::Ordering;
use std::{string::ToString, vec, vec::Vec};

use num_bigint::BigInt;

use super::{
  checked_add, checked_mul_pow10, checked_resize, checked_sub, cmp, div_rem, div_rem_pow10,
  is_positive, is_zero, le, max_magnitude, mul, mul_pow10, overflowing_add, overflowing_sub, pow10,
  push_digits, resize, round_div, write_digits,
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

/// A deterministic xorshift generator, so that a failure replays from the seed.
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    self.0 ^= self.0 >> 12;
    self.0 ^= self.0 << 25;
    self.0 ^= self.0 >> 27;
    self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
  }

  /// A nonzero value with a random number of significant limbs, each random, zero or all ones,
  /// and a top limb of random bit length, which gives every normalization shift.
  fn limbs<const N: usize>(&mut self) -> [u64; N] {
    let mut value = [0; N];
    let count = (self.next() % N as u64) as usize + 1;
    for limb in &mut value[..count] {
      *limb = match self.next() % 4 {
        0 => 0,
        1 => u64::MAX,
        _ => self.next(),
      };
    }
    value[count - 1] = (value[count - 1] >> (self.next() % 64)).max(1);
    value
  }
}

/// Checks `div_rem(a, b)` against `num-bigint`.
fn check_division<const N: usize>(a: [u64; N], b: [u64; N]) {
  let (quotient, remainder) = div_rem(a, b);
  let (big_a, big_b) = (bigint::from_limbs(&a, false), bigint::from_limbs(&b, false));
  assert_eq!(
    bigint::from_limbs(&quotient, false),
    &big_a / &big_b,
    "{a:x?} / {b:x?}"
  );
  assert_eq!(
    bigint::from_limbs(&remainder, false),
    &big_a % &big_b,
    "{a:x?} % {b:x?}"
  );
}

#[test]
fn division_matches_u64_and_u128() {
  let samples = bigint::thin(samples());
  for &a in &samples {
    for &b in samples.iter().filter(|&&b| b != 0) {
      if let (Ok(a), Ok(b)) = (u64::try_from(a), u64::try_from(b)) {
        assert_eq!(div_rem([a], [b]), ([a / b], [a % b]), "{a} / {b}");
      }
      assert_eq!(
        div_rem(from_u128(a), from_u128(b)),
        (from_u128(a / b), from_u128(a % b)),
        "{a} / {b}"
      );
    }
  }
}

fn division_against_big_integers<const N: usize>() {
  // Structured operands: limb boundaries, all-ones runs, single limbs, and a top limb with the high
  // bit set or with only the low bit.
  let mut operands = vec![[0; N], boundary::<N>(0), [u64::MAX; N]];
  for k in 1..N {
    operands.extend([boundary::<N>(k), ones_below::<N>(k)]);
    let mut top = ones_below::<N>(k);
    top[k] = 1 << 63;
    operands.push(top);
  }
  let mut seven = [0; N];
  seven[0] = 7;
  operands.push(seven);
  let operands = bigint::thin(operands);
  for &a in &operands {
    for &b in operands.iter().filter(|b| !is_zero(**b)) {
      check_division(a, b);
    }
  }
  let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ N as u64);
  let count = if cfg!(miri) { 8 } else { 3000 };
  for _ in 0..count {
    let b = rng.limbs::<N>();
    let a = match rng.next() % 4 {
      0 => [u64::MAX; N],
      _ => rng.limbs::<N>(),
    };
    check_division(a, b);
  }
}

#[test]
fn division_against_big_integers_for_every_width() {
  division_against_big_integers::<1>();
  division_against_big_integers::<2>();
  division_against_big_integers::<3>();
  division_against_big_integers::<4>();
  division_against_big_integers::<5>();
  division_against_big_integers::<6>();
  division_against_big_integers::<7>();
  division_against_big_integers::<8>();
}

/// The add-back step of Algorithm D, each reached exactly once and past the shortcut for a dividend
/// below the divisor. Both divisors have three significant limbs: with two, the correction by the
/// next divisor limb is exact and nothing is ever added back.
#[test]
fn division_adds_back() {
  const MAX: u64 = u64::MAX;
  // Two steps with shift 0. The second estimates 8, which passes the test with the second divisor
  // limb, zero, but the third divisor limb makes the subtraction borrow.
  let a = [MAX - 9, 7, 0, 4];
  let b = [MAX, 0, 1 << 63, 0];
  assert_eq!(div_rem(a, b), ([7, 0, 0, 0], [MAX - 2, 0, 1 << 63, 0]));
  check_division(a, b);
  // A divisor as wide as the dividend: one step, with shift 63, which estimates 2.
  let a = [MAX - 2, 1, 2];
  let b = [MAX, 0, 1];
  assert_eq!(div_rem(a, b), ([1, 0, 0], [MAX - 1, 0, 1]));
  check_division(a, b);
}

/// Estimates that start at `2^64` or `2^64 + 1`, when the top limb of the remaining dividend equals
/// the top divisor limb, and that stop correcting once the remainder of the estimate overflows a
/// limb.
#[test]
fn division_estimates_reach_two_pow_64() {
  const MAX: u64 = u64::MAX;
  const HIGH: u64 = 1 << 63;
  // The first step estimates 1 and corrects it to 0, leaving the top limb `HIGH`: the second
  // estimates `2^64`.
  let a = [0, 0, HIGH];
  let b = [MAX, HIGH, 0];
  assert_eq!(div_rem(a, b), ([MAX - 1, 0, 0], [MAX - 1, 2, 0]));
  check_division(a, b);
  // The same with `HIGH + 5` below the top limb: the second estimate is `2^64 + 1`.
  let a = [7, HIGH + 5, HIGH];
  assert_eq!(div_rem(a, b), ([MAX, 0, 0], [6, 7, 0]));
  check_division(a, b);
  // The remainder of the estimate overflows after one correction, so the loop stops before
  // comparing a truncated value.
  let a = [0x9a86_1c5d_6121_668a, 1, 0, 0x8000_0000_0000_0001];
  let b = [MAX, MAX - 1, 0, 0];
  assert_eq!(
    div_rem(a, b),
    (
      [0x8000_0000_0000_0002, 0x8000_0000_0000_0001, 0, 0],
      [0x1a86_1c5d_6121_668c, 5, 0, 0]
    )
  );
  check_division(a, b);
}

/// Divisors whose top limb has 0, 1, 31, 32, 62 and 63 leading zeros, including the shift of zero,
/// which shifts by nothing.
#[test]
fn division_for_every_normalization_shift() {
  for shift in [0, 1, 31, 32, 62, 63] {
    let top = u64::MAX >> shift;
    let b = [u64::MAX, 0, top];
    for a in [
      [u64::MAX; 3],
      b,
      [0, 0, top],
      [1, 0, top],
      [u64::MAX, u64::MAX, top],
    ] {
      check_division(a, b);
    }
    let mut wide = [u64::MAX; 8];
    wide[7] = top;
    check_division([u64::MAX; 8], wide);
    check_division([u64::MAX; 8], [u64::MAX, top, 0, 0, 0, 0, 0, 0]);
  }
}

fn division_edges<const N: usize>() {
  let one = boundary::<N>(0);
  let max = [u64::MAX; N];
  assert_eq!(div_rem(max, one), (max, [0; N]));
  assert_eq!(div_rem(max, max), (one, [0; N]));
  assert_eq!(div_rem([0; N], max), ([0; N], [0; N]));
  // A dividend below the divisor, and equal to it, for a divisor as wide as the dividend.
  let mut divisor = ones_below::<N>(N - 1);
  divisor[N - 1] = 1;
  let mut below = divisor;
  below[0] -= 1;
  assert_eq!(div_rem(below, divisor), ([0; N], below));
  assert_eq!(div_rem(divisor, divisor), (one, [0; N]));
  // A divisor with leading zero limbs and one significant limb takes the single-limb division.
  let mut seven = [0; N];
  seven[0] = 7;
  check_division(max, seven);
  // A divisor as wide as the dividend leaves one quotient limb.
  let mut wide = max;
  wide[N - 1] = 1;
  check_division(max, wide);
}

#[test]
fn division_edges_for_every_width() {
  division_edges::<2>();
  division_edges::<3>();
  division_edges::<4>();
  division_edges::<5>();
  division_edges::<6>();
  division_edges::<7>();
  division_edges::<8>();
}

/// Checks `round_div(value, divisor, mode)` against rounding by distance.
fn check_rounding<const N: usize>(value: [u64; N], divisor: [u64; N]) {
  let (big_value, big_divisor) = (
    bigint::from_limbs(&value, false),
    bigint::from_limbs(&divisor, false),
  );
  let truncated = &big_value / &big_divisor;
  for mode in bigint::MODES {
    let (rounded, remainder, away) = round_div(value, divisor, mode);
    let expected = bigint::round(&big_value, &big_divisor, mode);
    let rounded = bigint::from_limbs(&rounded, false);
    assert_eq!(rounded, expected, "{value:x?} / {divisor:x?}, {mode:?}");
    assert_eq!(
      bigint::from_limbs(&remainder, false),
      &big_value % &big_divisor
    );
    assert_eq!(away, rounded != truncated);
  }
}

fn rounding_division<const N: usize>() {
  let mut rng = Rng(0x5eed_f00d ^ N as u64);
  let count = if cfg!(miri) { 4 } else { 400 };
  for _ in 0..count {
    let divisor = rng.limbs::<N>();
    // Values at and around the multiples of the divisor and the halfway points between them.
    let big_divisor = bigint::from_limbs(&divisor, false);
    let quotient = bigint::from_limbs(&rng.limbs::<N>(), false) >> (64 * N as u32 / 2);
    let base = &quotient * &big_divisor;
    let half = &big_divisor / 2u8;
    for offset in [
      BigInt::ZERO,
      BigInt::from(1u8),
      &half - 1u8,
      half.clone(),
      &half + 1u8,
      &big_divisor - 1u8,
    ] {
      if let Some(value) = bigint::to_limbs::<N>(&(&base + offset), false) {
        check_rounding(value, divisor);
      }
    }
    check_rounding(rng.limbs::<N>(), divisor);
  }
  // Even divisors, whose halfway points are ties, with an even and an odd quotient.
  let mut ten = [0; N];
  ten[0] = 10;
  for value in [5, 15, 25, 4, 6, 14, 16] {
    let mut limbs = [0; N];
    limbs[0] = value;
    check_rounding(limbs, ten);
  }
  check_rounding([u64::MAX; N], boundary::<N>(N - 1));
}

#[test]
fn rounding_division_matches_rounding_by_distance() {
  rounding_division::<1>();
  rounding_division::<2>();
  rounding_division::<3>();
  rounding_division::<5>();
  rounding_division::<8>();
}
