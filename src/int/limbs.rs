//! Lower layer for `[u64; N]`: unsigned integers of `N` 64-bit limbs, least significant first.
//!
//! The functions mirror the native lower layer, generic over `N >= 1`: they have the same names
//! and contracts, except that some preconditions are weaker, as each function states. Where native
//! arithmetic would overflow on a violated precondition, they wrap modulo `2^(64N)` and fail a
//! debug assertion.

use core::cmp::Ordering;

use super::{DIGIT_BUF_LEN, DigitBuf};

/// `10^19`, the largest power of ten in a `u64`.
const TEN_POW_19: u64 = 10_000_000_000_000_000_000;

/// The exponent of `TEN_POW_19`: one limb operation scales by at most this many decimal digits.
const MAX_STEP: u32 = 19;

/// One.
const fn one<const N: usize>() -> [u64; N] {
  let mut one = [0; N];
  one[0] = 1;
  one
}

/// Returns `true` if `lhs` and `rhs` are equal.
pub(crate) const fn eq<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> bool {
  let mut i = 0;
  while i < N {
    if lhs[i] != rhs[i] {
      return false;
    }
    i += 1;
  }
  true
}

#[inline]
pub(crate) const fn is_zero<const N: usize>(value: [u64; N]) -> bool {
  eq(value, [0; N])
}

#[inline]
pub(crate) const fn is_positive<const N: usize>(value: [u64; N]) -> bool {
  !is_zero(value)
}

/// Compares the unsigned integers formed by the limbs below index `top`. Requires `top <= N`.
pub(crate) const fn cmp_below<const N: usize>(
  lhs: [u64; N],
  rhs: [u64; N],
  top: usize,
) -> Ordering {
  let mut i = top;
  while i > 0 {
    i -= 1;
    if lhs[i] != rhs[i] {
      return if lhs[i] < rhs[i] {
        Ordering::Less
      } else {
        Ordering::Greater
      };
    }
  }
  Ordering::Equal
}

/// Compares `lhs` and `rhs` from the most significant limb down.
#[inline]
pub(crate) const fn cmp<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> Ordering {
  cmp_below(lhs, rhs, N)
}

#[inline]
pub(crate) const fn le<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> bool {
  !matches!(cmp(lhs, rhs), Ordering::Greater)
}

/// `lhs + rhs` modulo `2^(64N)`, and whether the sum carried out of the top limb.
pub(crate) const fn overflowing_add<const N: usize>(
  lhs: [u64; N],
  rhs: [u64; N],
) -> ([u64; N], bool) {
  let mut sum = [0; N];
  let mut carry = false;
  let mut i = 0;
  while i < N {
    let (partial, first) = lhs[i].overflowing_add(rhs[i]);
    let (limb, second) = partial.overflowing_add(carry as u64);
    sum[i] = limb;
    carry = first || second;
    i += 1;
  }
  (sum, carry)
}

/// `lhs + rhs`, or `None` if the sum carries out of the top limb.
#[inline]
pub(crate) const fn checked_add<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> Option<[u64; N]> {
  match overflowing_add(lhs, rhs) {
    (sum, false) => Some(sum),
    (_, true) => None,
  }
}

/// `lhs - rhs` modulo `2^(64N)`, and whether the difference borrowed past the top limb.
pub(crate) const fn overflowing_sub<const N: usize>(
  lhs: [u64; N],
  rhs: [u64; N],
) -> ([u64; N], bool) {
  let mut difference = [0; N];
  let mut borrow = false;
  let mut i = 0;
  while i < N {
    let (partial, first) = lhs[i].overflowing_sub(rhs[i]);
    let (limb, second) = partial.overflowing_sub(borrow as u64);
    difference[i] = limb;
    borrow = first || second;
    i += 1;
  }
  (difference, borrow)
}

/// `lhs - rhs`, or `None` if `rhs > lhs`.
#[inline]
pub(crate) const fn checked_sub<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> Option<[u64; N]> {
  match overflowing_sub(lhs, rhs) {
    (difference, false) => Some(difference),
    (_, true) => None,
  }
}

/// `value * factor + addend` modulo `2^(64N)`, and the limb that carried out of the top limb.
const fn mul_add_u64<const N: usize>(value: [u64; N], factor: u64, addend: u64) -> ([u64; N], u64) {
  let mut product = [0; N];
  let mut carry = addend;
  let mut i = 0;
  while i < N {
    // At most `(2^64 - 1)^2 + 2^64 - 1 < 2^128`.
    let wide = value[i] as u128 * factor as u128 + carry as u128;
    product[i] = wide as u64;
    carry = (wide >> 64) as u64;
    i += 1;
  }
  (product, carry)
}

/// `lhs * rhs` modulo `2^(64N)`. Requires the product to be representable.
pub(crate) const fn mul<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> [u64; N] {
  let mut product = [0; N];
  let mut i = 0;
  while i < N {
    let mut carry: u64 = 0;
    let mut j = 0;
    while i + j < N {
      // At most `(2^64 - 1)^2 + 2 * (2^64 - 1) = 2^128 - 1`.
      let wide = lhs[i] as u128 * rhs[j] as u128 + product[i + j] as u128 + carry as u128;
      product[i + j] = wide as u64;
      carry = (wide >> 64) as u64;
      j += 1;
    }
    // A representable product carries nothing past the top limb, and the partial products that
    // would land there are zero.
    debug_assert!(carry == 0, "multiplication overflowed");
    while j < N {
      debug_assert!(lhs[i] == 0 || rhs[j] == 0, "multiplication overflowed");
      j += 1;
    }
    i += 1;
  }
  product
}

/// `value * 10^exp`, or `None` if the product is not representable.
pub(crate) const fn checked_mul_pow10<const N: usize>(
  mut value: [u64; N],
  mut exp: u32,
) -> Option<[u64; N]> {
  // Each partial product is at most the full product, so the first carry decides overflow.
  while exp > 0 {
    let step = if exp < MAX_STEP { exp } else { MAX_STEP };
    match mul_add_u64(value, 10u64.pow(step), 0) {
      (product, 0) => value = product,
      _ => return None,
    }
    exp -= step;
  }
  Some(value)
}

/// `value * 10^exp`. Requires the product to be representable.
pub(crate) const fn mul_pow10<const N: usize>(mut value: [u64; N], mut exp: u32) -> [u64; N] {
  while exp > 0 {
    let step = if exp < MAX_STEP { exp } else { MAX_STEP };
    let (product, carry) = mul_add_u64(value, 10u64.pow(step), 0);
    debug_assert!(carry == 0, "multiplication by a power of ten overflowed");
    value = product;
    exp -= step;
  }
  value
}

/// `10^exp`. Requires `10^exp` to be representable.
#[inline]
pub(crate) const fn pow10<const N: usize>(exp: u32) -> [u64; N] {
  mul_pow10(one(), exp)
}

/// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits` to be
/// representable.
#[inline]
pub(crate) const fn max_magnitude<const N: usize>(digits: u32) -> [u64; N] {
  // `10^digits >= 1`, so nothing borrows.
  overflowing_sub(pow10(digits), one()).0
}

/// Quotient and remainder of `value / divisor`. Requires `divisor != 0`.
pub(crate) const fn div_rem_u64<const N: usize>(value: [u64; N], divisor: u64) -> ([u64; N], u64) {
  let mut quotient = [0; N];
  let mut remainder: u64 = 0;
  let mut i = N;
  while i > 0 {
    i -= 1;
    // `remainder < divisor`, so each quotient limb fits in a `u64`.
    let wide = (remainder as u128) << 64 | value[i] as u128;
    quotient[i] = (wide / divisor as u128) as u64;
    remainder = (wide % divisor as u128) as u64;
  }
  (quotient, remainder)
}

/// Quotient and remainder of `value / 10^exp`.
pub(crate) const fn div_rem_pow10<const N: usize>(
  value: [u64; N],
  exp: u32,
) -> ([u64; N], [u64; N]) {
  // Dividing by the factors of `10^exp` in turn truncates once, as dividing by `10^exp` does.
  let mut quotient = value;
  let mut rest = exp;
  while rest > 0 {
    let step = if rest < MAX_STEP { rest } else { MAX_STEP };
    quotient = div_rem_u64(quotient, 10u64.pow(step)).0;
    rest -= step;
  }
  // `quotient * 10^exp <= value`, so neither the product nor the difference overflows.
  let (remainder, borrow) = overflowing_sub(value, mul_pow10(quotient, exp));
  debug_assert!(!borrow, "remainder of a division by a power of ten");
  (quotient, remainder)
}

/// Writes the decimal digits of `value` to the end of `buf`, without leading zeros (zero is a
/// single `0`), and returns the index of the first digit. Requires `value` to have at most
/// `DIGIT_BUF_LEN` digits, which holds for `N <= 5`.
pub(crate) const fn write_digits<const N: usize>(mut value: [u64; N], buf: &mut DigitBuf) -> usize {
  let mut start = DIGIT_BUF_LEN;
  // Divides off 19 digits at a time, least significant first.
  loop {
    let (quotient, mut chunk) = div_rem_u64(value, TEN_POW_19);
    value = quotient;
    if is_zero(value) {
      // The most significant chunk, without leading zeros.
      loop {
        start -= 1;
        buf[start] = b'0' + (chunk % 10) as u8;
        chunk /= 10;
        if chunk == 0 {
          return start;
        }
      }
    }
    // A less significant chunk has all 19 digits, including its leading zeros.
    let mut written = 0;
    while written < MAX_STEP {
      start -= 1;
      buf[start] = b'0' + (chunk % 10) as u8;
      chunk /= 10;
      written += 1;
    }
  }
}

/// `value * scale + chunk`. Requires a representable result.
const fn append<const N: usize>(value: [u64; N], scale: u64, chunk: u64) -> [u64; N] {
  let (appended, carry) = mul_add_u64(value, scale, chunk);
  debug_assert!(carry == 0, "appending digits overflowed");
  appended
}

/// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for each
/// digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires a
/// representable result.
pub(crate) const fn push_digits<const N: usize>(
  mut value: [u64; N],
  bytes: &[u8],
  start: usize,
  end: usize,
) -> [u64; N] {
  // Gathers up to 19 digits in a `u64` and appends them with one multi-limb multiplication. Each
  // intermediate value is a prefix of the result, so it is representable as well.
  let mut chunk: u64 = 0;
  let mut scale: u64 = 1;
  let mut i = start;
  while i < end {
    let byte = bytes[i];
    if byte.is_ascii_digit() {
      chunk = chunk * 10 + (byte - b'0') as u64;
      scale *= 10;
      if scale == TEN_POW_19 {
        value = append(value, scale, chunk);
        chunk = 0;
        scale = 1;
      }
    }
    i += 1;
  }
  append(value, scale, chunk)
}

/// Zero-extends or truncates `value` to `M` limbs.
pub(crate) const fn resize<const N: usize, const M: usize>(value: [u64; N]) -> [u64; M] {
  let mut resized = [0; M];
  let mut i = 0;
  while i < N && i < M {
    resized[i] = value[i];
    i += 1;
  }
  resized
}

/// `value` in `M` limbs, or `None` if it does not fit: a limb that truncation drops is not zero.
pub(crate) const fn checked_resize<const N: usize, const M: usize>(
  value: [u64; N],
) -> Option<[u64; M]> {
  let resized = resize::<N, M>(value);
  // Extending back restores every limb exactly when no dropped limb was nonzero.
  if eq(resize::<M, N>(resized), value) {
    Some(resized)
  } else {
    None
  }
}

#[cfg(test)]
mod tests;
