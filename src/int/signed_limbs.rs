//! Upper layer for `[u64; N]`: two's complement integers of `N` limbs of `B = 64` bits, least
//! significant first.
//!
//! The top bit of the top limb is the sign. The functions mirror the native upper layer, generic
//! over `N >= 1`: they have the same names and contracts, except that some preconditions are
//! weaker, as each function states, and they handle the sign and pass magnitude work to the lower
//! layer through `unsigned_abs`. "Representable" means within the signed range, from
//! `MIN = -2^(B * N - 1)` to `2^(B * N - 1) - 1`.

use core::cmp::Ordering;

use super::{DigitBuf, limbs};
use crate::rounding::RoundingMode;

/// The type of a limb: an unsigned integer of `B` bits.
type Limb = u64;

/// The signed integer of `B` bits, which reads the top limb with its sign.
type SignedLimb = i64;

#[inline]
pub(crate) const fn is_negative<const N: usize>(value: &[Limb; N]) -> bool {
  (value[N - 1] as SignedLimb) < 0
}

#[inline]
pub(crate) const fn is_zero<const N: usize>(value: &[Limb; N]) -> bool {
  limbs::is_zero(value)
}

#[inline]
pub(crate) const fn is_positive<const N: usize>(value: &[Limb; N]) -> bool {
  !is_negative(value) && !is_zero(value)
}

/// Compares `lhs` and `rhs`: the top limbs as `SignedLimb`, then the others as `Limb`, most
/// significant first.
pub(crate) const fn cmp<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> Ordering {
  let lhs_top = lhs[N - 1] as SignedLimb;
  let rhs_top = rhs[N - 1] as SignedLimb;
  if lhs_top < rhs_top {
    Ordering::Less
  } else if lhs_top > rhs_top {
    Ordering::Greater
  } else {
    limbs::cmp_below(lhs, rhs, N - 1)
  }
}

#[inline]
pub(crate) const fn le<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> bool {
  !matches!(cmp(lhs, rhs), Ordering::Greater)
}

/// `lhs + rhs`, or `None` if the sum is not representable.
pub(crate) const fn checked_add<const N: usize>(
  lhs: &[Limb; N],
  rhs: &[Limb; N],
) -> Option<[Limb; N]> {
  let (sum, _) = limbs::overflowing_add(lhs, rhs);
  // The sum overflows exactly when the operands have the same sign and the wrapped sum
  // the other.
  if is_negative(lhs) == is_negative(rhs) && is_negative(&sum) != is_negative(lhs) {
    None
  } else {
    Some(sum)
  }
}

/// `lhs - rhs`, or `None` if the difference is not representable.
pub(crate) const fn checked_sub<const N: usize>(
  lhs: &[Limb; N],
  rhs: &[Limb; N],
) -> Option<[Limb; N]> {
  let (difference, _) = limbs::overflowing_sub(lhs, rhs);
  // The difference overflows exactly when the operands have different signs and the wrapped
  // difference has the sign of `rhs`.
  if is_negative(lhs) != is_negative(rhs) && is_negative(&difference) != is_negative(lhs) {
    None
  } else {
    Some(difference)
  }
}

/// `-value` modulo `2^(B * N)`: `!value + 1`, which maps `MIN` to itself.
const fn wrapping_neg<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
  let mut negated = [0; N];
  let mut carry = true;
  let mut i = 0;
  while i < N {
    let (limb, overflow) = (!value[i]).overflowing_add(carry as Limb);
    negated[i] = limb;
    carry = overflow;
    i += 1;
  }
  negated
}

/// `-value`. Requires `value != MIN`.
#[inline]
pub(crate) const fn neg<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
  let negated = wrapping_neg(value);
  debug_assert!(
    limbs::is_zero(value) || is_negative(value) != is_negative(&negated),
    "negation overflowed"
  );
  negated
}

/// `|value|`. Requires `value != MIN`.
#[inline]
pub(crate) const fn abs<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
  if is_negative(value) {
    neg(value)
  } else {
    *value
  }
}

/// `|value|` as an unsigned integer, which also holds `|MIN| = 2^(B * N - 1)`.
#[inline]
pub(crate) const fn unsigned_abs<const N: usize>(value: &[Limb; N]) -> [Limb; N] {
  if is_negative(value) {
    wrapping_neg(value)
  } else {
    *value
  }
}

/// The value with the magnitude `magnitude`, negated if `negative`, or `None` if it is not
/// representable.
const fn checked_from_magnitude<const N: usize>(
  magnitude: &[Limb; N],
  negative: bool,
) -> Option<[Limb; N]> {
  if !negative {
    return if is_negative(magnitude) {
      None
    } else {
      Some(*magnitude)
    };
  }
  // A magnitude from 1 to `2^(B * N - 1)` negates to a value with the sign bit set.
  let negated = wrapping_neg(magnitude);
  if limbs::is_zero(magnitude) || is_negative(&negated) {
    Some(negated)
  } else {
    None
  }
}

/// The value with the magnitude `magnitude`, negated if `negative`. Requires it to be
/// representable.
#[inline]
const fn from_magnitude<const N: usize>(magnitude: &[Limb; N], negative: bool) -> [Limb; N] {
  debug_assert!(
    checked_from_magnitude(magnitude, negative).is_some(),
    "signed result overflowed"
  );
  if negative {
    wrapping_neg(magnitude)
  } else {
    *magnitude
  }
}

/// `10^exp`. Requires `10^exp` to be representable.
#[inline]
pub(crate) const fn pow10<const N: usize>(exp: u32) -> [Limb; N] {
  from_magnitude(&limbs::pow10(exp), false)
}

/// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits`
/// to be representable.
#[inline]
pub(crate) const fn max_magnitude<const N: usize>(digits: u32) -> [Limb; N] {
  from_magnitude(&limbs::max_magnitude(digits), false)
}

/// `lhs * rhs`. Requires the product to be representable.
#[inline]
pub(crate) const fn mul<const N: usize>(lhs: &[Limb; N], rhs: &[Limb; N]) -> [Limb; N] {
  let magnitude = limbs::mul(&unsigned_abs(lhs), &unsigned_abs(rhs));
  from_magnitude(&magnitude, is_negative(lhs) != is_negative(rhs))
}

/// `value * 10^exp`. Requires the product to be representable.
#[inline]
pub(crate) const fn mul_pow10<const N: usize>(value: &[Limb; N], exp: u32) -> [Limb; N] {
  from_magnitude(
    &limbs::mul_pow10(&unsigned_abs(value), exp),
    is_negative(value),
  )
}

/// `value * 10^exp`, or `None` if the product is not representable.
pub(crate) const fn checked_mul_pow10<const N: usize>(
  value: &[Limb; N],
  exp: u32,
) -> Option<[Limb; N]> {
  match limbs::checked_mul_pow10(&unsigned_abs(value), exp) {
    Some(magnitude) => checked_from_magnitude(&magnitude, is_negative(value)),
    None => None,
  }
}

/// Quotient and remainder of `value / 10^exp`, rounding toward zero. As with `/` and `%`, both
/// have the sign of `value`.
pub(crate) const fn div_rem_pow10<const N: usize>(
  value: &[Limb; N],
  exp: u32,
) -> ([Limb; N], [Limb; N]) {
  let (quotient, remainder) = limbs::div_rem_pow10(&unsigned_abs(value), exp);
  // Both magnitudes are at most `|value|`, so each is exact with the sign of `value`.
  let negative = is_negative(value);
  (
    from_magnitude(&quotient, negative),
    from_magnitude(&remainder, negative),
  )
}

/// Quotient and remainder of `lhs / rhs`, rounding toward zero. As with `/` and `%`, the
/// remainder has the sign of `lhs`. Requires `rhs != 0`, and `rhs != -1` if `lhs == MIN`.
pub(crate) const fn div_rem<const N: usize>(
  lhs: &[Limb; N],
  rhs: &[Limb; N],
) -> ([Limb; N], [Limb; N]) {
  // The magnitudes are unsigned, which also holds `|MIN|`. The quotient's magnitude is at
  // most `|lhs|`, and the remainder's below `|rhs|`.
  let (quotient, remainder) = limbs::div_rem(&unsigned_abs(lhs), &unsigned_abs(rhs));
  (
    from_magnitude(&quotient, is_negative(lhs) != is_negative(rhs)),
    from_magnitude(&remainder, is_negative(lhs)),
  )
}

/// `value / divisor` rounded to an integer with `mode`, the remainder of the division rounded
/// toward zero, which has the sign of `value`, and whether `mode` rounded the quotient away
/// from zero. Requires `divisor > 0`.
pub(crate) const fn round_div<const N: usize>(
  value: &[Limb; N],
  divisor: &[Limb; N],
  mode: RoundingMode,
) -> ([Limb; N], [Limb; N], bool) {
  let (quotient, remainder) = div_rem(value, divisor);
  // The sign of the value, which the quotient loses when it is zero. A positive divisor is its
  // own magnitude.
  let negative = is_negative(value);
  let away = limbs::rounds_away(
    &unsigned_abs(&quotient),
    &unsigned_abs(&remainder),
    divisor,
    negative,
    mode,
  );
  // Rounding away needs a nonzero remainder, so the divisor is at least 2 and `|quotient|` at
  // most half of `|value|`: one step further from zero is representable, so the wrapping
  // arithmetic is exact.
  let rounded = if !away {
    quotient
  } else if negative {
    limbs::overflowing_sub(&quotient, &limbs::one()).0
  } else {
    limbs::overflowing_add(&quotient, &limbs::one()).0
  };
  (rounded, remainder, away)
}

/// Writes the decimal digits of `|value|` to the end of `buf`, without leading zeros (zero is a
/// single `0`), and returns the index of the first digit. Requires `|value|` to have at most
/// `DIGIT_BUF_LEN` digits, which holds for `B * N <= 512`.
#[inline]
pub(crate) const fn write_digits<const N: usize>(value: &[Limb; N], buf: &mut DigitBuf) -> usize {
  limbs::write_digits(&unsigned_abs(value), buf)
}

/// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for
/// each digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires
/// `value` not to be negative and a representable result.
#[inline]
pub(crate) const fn push_digits<const N: usize>(
  value: &[Limb; N],
  bytes: &[u8],
  start: usize,
  end: usize,
) -> [Limb; N] {
  from_magnitude(&limbs::push_digits(value, bytes, start, end), false)
}

/// Sign-extends or truncates `value` to `M` limbs.
pub(crate) const fn resize<const N: usize, const M: usize>(value: &[Limb; N]) -> [Limb; M] {
  let fill = if is_negative(value) { Limb::MAX } else { 0 };
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
  value: &[Limb; N],
) -> Option<[Limb; M]> {
  let resized = resize::<N, M>(value);
  // Sign-extending back restores every limb exactly when truncation dropped only copies of the
  // sign bit that remains.
  if limbs::eq(&resize::<M, N>(&resized), value) {
    Some(resized)
  } else {
    None
  }
}

/// `value` with every bit at index `bits - 1` or above set to bit `bits - 1`: the two's
/// complement integer of `bits` bits in the low bits of `value`, sign-extended to `N` limbs.
/// Requires `1 <= bits <= B * N`.
pub(crate) const fn sign_extend<const N: usize>(value: &[Limb; N], bits: u32) -> [Limb; N] {
  let top = ((bits - 1) / Limb::BITS) as usize;
  // Shifting bit `bits - 1` to the top of its limb and back with an arithmetic shift copies it
  // into the bits above.
  let shift = Limb::BITS - 1 - (bits - 1) % Limb::BITS;
  let mut extended = *value;
  extended[top] = (((value[top] << shift) as SignedLimb) >> shift) as Limb;
  let fill = if (extended[top] as SignedLimb) < 0 {
    Limb::MAX
  } else {
    0
  };
  let mut i = top + 1;
  while i < N {
    extended[i] = fill;
    i += 1;
  }
  extended
}

/// Returns `true` if `value` is within the range of `bits` bits, from `-2^(bits - 1)` to
/// `2^(bits - 1) - 1`. Requires `1 <= bits <= B * N`.
#[inline]
pub(crate) const fn fits_bits<const N: usize>(value: &[Limb; N], bits: u32) -> bool {
  // Sign-extending from bit `bits - 1` changes only a value outside the range.
  limbs::eq(&sign_extend(value, bits), value)
}

/// `2^(bits - 1) - 1`, the largest value of `bits` bits. Requires `1 <= bits <= B * N`.
#[inline]
pub(crate) const fn max_of_bits<const N: usize>(bits: u32) -> [Limb; N] {
  limbs::max_of_bits(bits - 1)
}

/// `-2^(bits - 1)`, the smallest value of `bits` bits. Requires `1 <= bits <= B * N`.
pub(crate) const fn min_of_bits<const N: usize>(bits: u32) -> [Limb; N] {
  // Every bit from index `bits - 1` up is set, and every bit below it is clear.
  let mut min = max_of_bits::<N>(bits);
  let mut i = 0;
  while i < N {
    min[i] = !min[i];
    i += 1;
  }
  min
}

/// The unsigned integer `value` in `M` limbs of two's complement, or `None` if it is not
/// representable.
pub(crate) const fn checked_from_unsigned<const N: usize, const M: usize>(
  value: &[Limb; N],
) -> Option<[Limb; M]> {
  match limbs::checked_resize::<N, M>(value) {
    Some(resized) if !is_negative(&resized) => Some(resized),
    _ => None,
  }
}

/// `value` as an unsigned integer of `M` limbs, or `None` if it is negative or does not fit.
pub(crate) const fn checked_to_unsigned<const N: usize, const M: usize>(
  value: &[Limb; N],
) -> Option<[Limb; M]> {
  if is_negative(value) {
    None
  } else {
    limbs::checked_resize(value)
  }
}

#[cfg(test)]
mod tests;
