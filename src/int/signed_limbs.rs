//! Upper layer for `[u64; N]`: two's complement integers of `N` 64-bit limbs, least significant
//! first.
//!
//! The top bit of the top limb is the sign. The functions mirror the native upper layer, generic
//! over `N >= 1`: they have the same names and contracts, except that some preconditions are
//! weaker, as each function states, and they handle the sign and pass magnitude work to the lower
//! layer through `unsigned_abs`. "Representable" means within the signed range, from
//! `MIN = -2^(64N - 1)` to `2^(64N - 1) - 1`.

use core::cmp::Ordering;

use super::{DigitBuf, limbs};

#[inline]
pub(crate) const fn is_negative<const N: usize>(value: [u64; N]) -> bool {
  (value[N - 1] as i64) < 0
}

#[inline]
pub(crate) const fn is_zero<const N: usize>(value: [u64; N]) -> bool {
  limbs::is_zero(value)
}

#[inline]
pub(crate) const fn is_positive<const N: usize>(value: [u64; N]) -> bool {
  !is_negative(value) && !is_zero(value)
}

/// Compares `lhs` and `rhs`: the top limbs as `i64`, then the others as `u64`, most significant
/// first.
pub(crate) const fn cmp<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> Ordering {
  let lhs_top = lhs[N - 1] as i64;
  let rhs_top = rhs[N - 1] as i64;
  if lhs_top < rhs_top {
    Ordering::Less
  } else if lhs_top > rhs_top {
    Ordering::Greater
  } else {
    limbs::cmp_below(lhs, rhs, N - 1)
  }
}

#[inline]
pub(crate) const fn le<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> bool {
  !matches!(cmp(lhs, rhs), Ordering::Greater)
}

/// `lhs + rhs`, or `None` if the sum is not representable.
pub(crate) const fn checked_add<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> Option<[u64; N]> {
  let (sum, _) = limbs::overflowing_add(lhs, rhs);
  // The sum overflows exactly when the operands have the same sign and the wrapped sum the other.
  if is_negative(lhs) == is_negative(rhs) && is_negative(sum) != is_negative(lhs) {
    None
  } else {
    Some(sum)
  }
}

/// `lhs - rhs`, or `None` if the difference is not representable.
pub(crate) const fn checked_sub<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> Option<[u64; N]> {
  let (difference, _) = limbs::overflowing_sub(lhs, rhs);
  // The difference overflows exactly when the operands have different signs and the wrapped
  // difference has the sign of `rhs`.
  if is_negative(lhs) != is_negative(rhs) && is_negative(difference) != is_negative(lhs) {
    None
  } else {
    Some(difference)
  }
}

/// `-value` modulo `2^(64N)`: `!value + 1`, which maps `MIN` to itself.
const fn wrapping_neg<const N: usize>(value: [u64; N]) -> [u64; N] {
  let mut negated = [0; N];
  let mut carry = true;
  let mut i = 0;
  while i < N {
    let (limb, overflow) = (!value[i]).overflowing_add(carry as u64);
    negated[i] = limb;
    carry = overflow;
    i += 1;
  }
  negated
}

/// `-value`. Requires `value != MIN`.
#[inline]
pub(crate) const fn neg<const N: usize>(value: [u64; N]) -> [u64; N] {
  let negated = wrapping_neg(value);
  debug_assert!(
    limbs::is_zero(value) || is_negative(value) != is_negative(negated),
    "negation overflowed"
  );
  negated
}

/// `|value|`. Requires `value != MIN`.
#[inline]
pub(crate) const fn abs<const N: usize>(value: [u64; N]) -> [u64; N] {
  if is_negative(value) {
    neg(value)
  } else {
    value
  }
}

/// `|value|` as an unsigned integer, which also holds `|MIN| = 2^(64N - 1)`.
#[inline]
pub(crate) const fn unsigned_abs<const N: usize>(value: [u64; N]) -> [u64; N] {
  if is_negative(value) {
    wrapping_neg(value)
  } else {
    value
  }
}

/// The value with the magnitude `magnitude`, negated if `negative`, or `None` if it is not
/// representable.
const fn checked_from_magnitude<const N: usize>(
  magnitude: [u64; N],
  negative: bool,
) -> Option<[u64; N]> {
  if !negative {
    return if is_negative(magnitude) {
      None
    } else {
      Some(magnitude)
    };
  }
  // A magnitude from 1 to `2^(64N - 1)` negates to a value with the sign bit set.
  let negated = wrapping_neg(magnitude);
  if limbs::is_zero(magnitude) || is_negative(negated) {
    Some(negated)
  } else {
    None
  }
}

/// The value with the magnitude `magnitude`, negated if `negative`. Requires it to be
/// representable.
#[inline]
const fn from_magnitude<const N: usize>(magnitude: [u64; N], negative: bool) -> [u64; N] {
  debug_assert!(
    checked_from_magnitude(magnitude, negative).is_some(),
    "signed result overflowed"
  );
  if negative {
    wrapping_neg(magnitude)
  } else {
    magnitude
  }
}

/// `10^exp`. Requires `10^exp` to be representable.
#[inline]
pub(crate) const fn pow10<const N: usize>(exp: u32) -> [u64; N] {
  from_magnitude(limbs::pow10(exp), false)
}

/// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits` to be
/// representable.
#[inline]
pub(crate) const fn max_magnitude<const N: usize>(digits: u32) -> [u64; N] {
  from_magnitude(limbs::max_magnitude(digits), false)
}

/// `lhs * rhs`. Requires the product to be representable.
#[inline]
pub(crate) const fn mul<const N: usize>(lhs: [u64; N], rhs: [u64; N]) -> [u64; N] {
  let magnitude = limbs::mul(unsigned_abs(lhs), unsigned_abs(rhs));
  from_magnitude(magnitude, is_negative(lhs) != is_negative(rhs))
}

/// `value * 10^exp`. Requires the product to be representable.
#[inline]
pub(crate) const fn mul_pow10<const N: usize>(value: [u64; N], exp: u32) -> [u64; N] {
  from_magnitude(
    limbs::mul_pow10(unsigned_abs(value), exp),
    is_negative(value),
  )
}

/// `value * 10^exp`, or `None` if the product is not representable.
pub(crate) const fn checked_mul_pow10<const N: usize>(
  value: [u64; N],
  exp: u32,
) -> Option<[u64; N]> {
  match limbs::checked_mul_pow10(unsigned_abs(value), exp) {
    Some(magnitude) => checked_from_magnitude(magnitude, is_negative(value)),
    None => None,
  }
}

/// Quotient and remainder of `value / 10^exp`, rounding toward zero. As with `/` and `%`, both
/// have the sign of `value`.
pub(crate) const fn div_rem_pow10<const N: usize>(
  value: [u64; N],
  exp: u32,
) -> ([u64; N], [u64; N]) {
  let (quotient, remainder) = limbs::div_rem_pow10(unsigned_abs(value), exp);
  // Both magnitudes are at most `|value|`, so each is exact with the sign of `value`.
  let negative = is_negative(value);
  (
    from_magnitude(quotient, negative),
    from_magnitude(remainder, negative),
  )
}

/// Writes the decimal digits of `|value|` to the end of `buf`, without leading zeros (zero is a
/// single `0`), and returns the index of the first digit. Requires `|value|` to have at most
/// `DIGIT_BUF_LEN` digits, which holds for `N <= 5`.
#[inline]
pub(crate) const fn write_digits<const N: usize>(value: [u64; N], buf: &mut DigitBuf) -> usize {
  limbs::write_digits(unsigned_abs(value), buf)
}

/// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for each
/// digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires `value`
/// not to be negative and a representable result.
#[inline]
pub(crate) const fn push_digits<const N: usize>(
  value: [u64; N],
  bytes: &[u8],
  start: usize,
  end: usize,
) -> [u64; N] {
  from_magnitude(limbs::push_digits(value, bytes, start, end), false)
}

/// Sign-extends or truncates `value` to `M` limbs.
pub(crate) const fn resize<const N: usize, const M: usize>(value: [u64; N]) -> [u64; M] {
  let fill = if is_negative(value) { u64::MAX } else { 0 };
  let mut resized = [fill; M];
  let mut i = 0;
  while i < N && i < M {
    resized[i] = value[i];
    i += 1;
  }
  resized
}

/// `value` in `M` limbs, or `None` if it is not representable in `M` limbs.
pub(crate) const fn checked_resize<const N: usize, const M: usize>(
  value: [u64; N],
) -> Option<[u64; M]> {
  let resized = resize::<N, M>(value);
  // Sign-extending back restores every limb exactly when truncation dropped only copies of the
  // sign bit that remains.
  if limbs::eq(resize::<M, N>(resized), value) {
    Some(resized)
  } else {
    None
  }
}

/// The unsigned integer `value` in `M` limbs of two's complement, or `None` if it is not
/// representable.
pub(crate) const fn checked_from_unsigned<const N: usize, const M: usize>(
  value: [u64; N],
) -> Option<[u64; M]> {
  match limbs::checked_resize::<N, M>(value) {
    Some(resized) if !is_negative(resized) => Some(resized),
    _ => None,
  }
}

/// `value` as an unsigned integer of `M` limbs, or `None` if it is negative or does not fit.
pub(crate) const fn checked_to_unsigned<const N: usize, const M: usize>(
  value: [u64; N],
) -> Option<[u64; M]> {
  if is_negative(value) {
    None
  } else {
    limbs::checked_resize(value)
  }
}

#[cfg(test)]
mod tests;
