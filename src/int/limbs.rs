//! Lower layer for `[u64; N]`: unsigned integers of `N` 64-bit limbs, least significant first.
//!
//! The functions mirror the native lower layer, generic over `N >= 1`: they have the same names
//! and contracts, except that some preconditions are weaker, as each function states. Where native
//! arithmetic would overflow on a violated precondition, they wrap modulo `2^(64N)` and fail a
//! debug assertion.

use core::cmp::Ordering;

use super::DigitBuf;
use crate::rounding::{self, RoundingMode};

/// `10^19`, the largest power of ten in a `u64`.
const TEN_POW_19: u64 = 10_000_000_000_000_000_000;

/// The exponent of `TEN_POW_19`: one limb operation scales by at most this many decimal digits.
const MAX_STEP: u32 = 19;

/// `a * b + carry + add` as two limbs, the low one first. It is at most
/// `(2^64 - 1)^2 + 2 * (2^64 - 1) = 2^128 - 1`, so it does not overflow.
#[inline]
const fn carrying_mul_add(a: u64, b: u64, carry: u64, add: u64) -> (u64, u64) {
  let wide = a as u128 * b as u128 + carry as u128 + add as u128;
  (wide as u64, (wide >> 64) as u64)
}

/// Quotient and remainder of `high * 2^64 + low` by `divisor`. Requires `high < divisor`, so that
/// the quotient fits a limb. A zero divisor fails as native division does.
#[inline]
const fn div_rem_wide(high: u64, low: u64, divisor: u64) -> (u64, u64) {
  debug_assert!(
    divisor == 0 || high < divisor,
    "the quotient overflows a limb"
  );
  let wide = (high as u128) << 64 | low as u128;
  (
    (wide / divisor as u128) as u64,
    (wide % divisor as u128) as u64,
  )
}

/// One.
pub(crate) const fn one<const N: usize>() -> [u64; N] {
  let mut one = [0; N];
  one[0] = 1;
  one
}

/// Returns `true` if `lhs` and `rhs` are equal.
pub(crate) const fn eq<const N: usize>(lhs: &[u64; N], rhs: &[u64; N]) -> bool {
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
pub(crate) const fn is_zero<const N: usize>(value: &[u64; N]) -> bool {
  eq(value, &[0; N])
}

#[inline]
pub(crate) const fn is_positive<const N: usize>(value: &[u64; N]) -> bool {
  !is_zero(value)
}

/// Compares the unsigned integers formed by the limbs below index `top`. Requires `top <= N`.
pub(crate) const fn cmp_below<const N: usize>(
  lhs: &[u64; N],
  rhs: &[u64; N],
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
pub(crate) const fn cmp<const N: usize>(lhs: &[u64; N], rhs: &[u64; N]) -> Ordering {
  cmp_below(lhs, rhs, N)
}

#[inline]
pub(crate) const fn le<const N: usize>(lhs: &[u64; N], rhs: &[u64; N]) -> bool {
  !matches!(cmp(lhs, rhs), Ordering::Greater)
}

/// `lhs + rhs` modulo `2^(64N)`, and whether the sum carried out of the top limb.
pub(crate) const fn overflowing_add<const N: usize>(
  lhs: &[u64; N],
  rhs: &[u64; N],
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
pub(crate) const fn checked_add<const N: usize>(
  lhs: &[u64; N],
  rhs: &[u64; N],
) -> Option<[u64; N]> {
  match overflowing_add(lhs, rhs) {
    (sum, false) => Some(sum),
    (_, true) => None,
  }
}

/// `lhs - rhs` modulo `2^(64N)`, and whether the difference borrowed past the top limb.
pub(crate) const fn overflowing_sub<const N: usize>(
  lhs: &[u64; N],
  rhs: &[u64; N],
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
pub(crate) const fn checked_sub<const N: usize>(
  lhs: &[u64; N],
  rhs: &[u64; N],
) -> Option<[u64; N]> {
  match overflowing_sub(lhs, rhs) {
    (difference, false) => Some(difference),
    (_, true) => None,
  }
}

/// `value * factor + addend` modulo `2^(64N)`, and the limb that carried out of the top limb.
const fn mul_add_limb<const N: usize>(
  value: &[u64; N],
  factor: u64,
  addend: u64,
) -> ([u64; N], u64) {
  let mut product = [0; N];
  let mut carry = addend;
  let mut i = 0;
  while i < N {
    (product[i], carry) = carrying_mul_add(value[i], factor, carry, 0);
    i += 1;
  }
  (product, carry)
}

/// `lhs * rhs` modulo `2^(64N)`. Requires the product to be representable.
pub(crate) const fn mul<const N: usize>(lhs: &[u64; N], rhs: &[u64; N]) -> [u64; N] {
  let mut product = [0; N];
  let mut i = 0;
  while i < N {
    let mut carry: u64 = 0;
    let mut j = 0;
    while i + j < N {
      (product[i + j], carry) = carrying_mul_add(lhs[i], rhs[j], carry, product[i + j]);
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
  value: &[u64; N],
  mut exp: u32,
) -> Option<[u64; N]> {
  let mut value = *value;
  // Each partial product is at most the full product, so the first carry decides overflow.
  while exp > 0 {
    let step = if exp < MAX_STEP { exp } else { MAX_STEP };
    match mul_add_limb(&value, 10u64.pow(step), 0) {
      (product, 0) => value = product,
      _ => return None,
    }
    exp -= step;
  }
  Some(value)
}

/// `value * 10^exp`. Requires the product to be representable.
pub(crate) const fn mul_pow10<const N: usize>(value: &[u64; N], mut exp: u32) -> [u64; N] {
  let mut value = *value;
  while exp > 0 {
    let step = if exp < MAX_STEP { exp } else { MAX_STEP };
    let (product, carry) = mul_add_limb(&value, 10u64.pow(step), 0);
    debug_assert!(carry == 0, "multiplication by a power of ten overflowed");
    value = product;
    exp -= step;
  }
  value
}

/// `10^exp`. Requires `10^exp` to be representable.
#[inline]
pub(crate) const fn pow10<const N: usize>(exp: u32) -> [u64; N] {
  mul_pow10(&one(), exp)
}

/// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits` to be
/// representable.
#[inline]
pub(crate) const fn max_magnitude<const N: usize>(digits: u32) -> [u64; N] {
  // `10^digits >= 1`, so nothing borrows.
  overflowing_sub(&pow10(digits), &one()).0
}

/// Quotient and remainder of `value / divisor`. Requires `divisor != 0`.
pub(crate) const fn div_rem_limb<const N: usize>(
  value: &[u64; N],
  divisor: u64,
) -> ([u64; N], u64) {
  let mut quotient = [0; N];
  let mut remainder: u64 = 0;
  let mut i = N;
  while i > 0 {
    i -= 1;
    // `remainder < divisor`, as `div_rem_wide` requires.
    (quotient[i], remainder) = div_rem_wide(remainder, value[i], divisor);
  }
  (quotient, remainder)
}

/// Quotient and remainder of `value / 10^exp`.
pub(crate) const fn div_rem_pow10<const N: usize>(
  value: &[u64; N],
  exp: u32,
) -> ([u64; N], [u64; N]) {
  // Dividing by the factors of `10^exp` in turn truncates once, as dividing by `10^exp` does.
  let mut quotient = *value;
  let mut rest = exp;
  while rest > 0 {
    let step = if rest < MAX_STEP { rest } else { MAX_STEP };
    quotient = div_rem_limb(&quotient, 10u64.pow(step)).0;
    rest -= step;
  }
  // `quotient * 10^exp <= value`, so neither the product nor the difference overflows.
  let (remainder, borrow) = overflowing_sub(value, &mul_pow10(&quotient, exp));
  debug_assert!(!borrow, "remainder of a division by a power of ten");
  (quotient, remainder)
}

/// The dividend of `div_rem` once normalized: `N` limbs, and at index `N` the limb that
/// normalization shifts out of the top one. It stands for a `[u64; N + 1]`, which stable Rust
/// cannot name for a generic `N`. The division reduces it to the normalized remainder.
struct Dividend<const N: usize> {
  limbs: [u64; N],
  top: u64,
}

impl<const N: usize> Dividend<N> {
  /// The limb at index `i`. Requires `i <= N`.
  const fn get(&self, i: usize) -> u64 {
    if i == N { self.top } else { self.limbs[i] }
  }

  /// Sets the limb at index `i`. Requires `i <= N`.
  const fn set(&mut self, i: usize, limb: u64) {
    if i == N {
      self.top = limb;
    } else {
      self.limbs[i] = limb;
    }
  }
}

/// `value << shift` modulo `2^(64N)`, and the bits shifted out of the top limb. Requires
/// `shift < 64`.
const fn shl<const N: usize>(value: &[u64; N], shift: u32) -> ([u64; N], u64) {
  // The carry below would shift by 64 bits, which overflows.
  if shift == 0 {
    return (*value, 0);
  }
  let mut shifted = [0; N];
  let mut carry = 0;
  let mut i = 0;
  while i < N {
    shifted[i] = value[i] << shift | carry;
    carry = value[i] >> (64 - shift);
    i += 1;
  }
  (shifted, carry)
}

/// `value >> shift`. Requires `shift < 64`.
const fn shr<const N: usize>(value: &[u64; N], shift: u32) -> [u64; N] {
  // The bits taken from the next limb below would be shifted by 64 bits, which overflows.
  if shift == 0 {
    return *value;
  }
  let mut shifted = [0; N];
  let mut i = 0;
  while i < N {
    shifted[i] = value[i] >> shift;
    if i + 1 < N {
      shifted[i] |= value[i + 1] << (64 - shift);
    }
    i += 1;
  }
  shifted
}

/// Quotient and remainder of `lhs / rhs`. Requires `rhs != 0`.
///
/// A divisor of one significant limb goes through `div_rem_limb`, and a dividend below the divisor
/// has the quotient zero. Otherwise this is Algorithm D of Knuth's The Art of Computer
/// Programming, volume 2, section 4.3.1: both operands are shifted left until the high bit of the
/// divisor's top limb is set, each quotient limb is estimated from the top limbs of the remaining
/// dividend and corrected, and the remainder is shifted back.
pub(crate) const fn div_rem<const N: usize>(
  lhs: &[u64; N],
  rhs: &[u64; N],
) -> ([u64; N], [u64; N]) {
  // The number of significant limbs of the divisor, at least one.
  let mut n = N;
  while n > 1 && rhs[n - 1] == 0 {
    n -= 1;
  }
  if n == 1 {
    // Algorithm D needs two divisor limbs. A zero divisor fails here as native division does.
    let (quotient, remainder) = div_rem_limb(lhs, rhs[0]);
    let mut wide = [0; N];
    wide[0] = remainder;
    return (quotient, wide);
  }
  if matches!(cmp(lhs, rhs), Ordering::Less) {
    return ([0; N], *lhs);
  }

  // D1: normalize. The shift drops only zero bits of the divisor, while the dividend's top bits
  // move to an extra limb. With the high bit of the divisor's top limb set, each estimate below
  // exceeds its quotient limb by at most two.
  let shift = rhs[n - 1].leading_zeros();
  let (divisor, lost) = shl(rhs, shift);
  debug_assert!(lost == 0, "normalization dropped divisor bits");
  let (limbs, top) = shl(lhs, shift);
  let mut rem = Dividend { limbs, top };
  let high = divisor[n - 1];
  let next = divisor[n - 2];
  let mut quotient = [0; N];
  // D2: one step per quotient limb, from the most significant down. Step `j` divides the window
  // `rem[j..=j + n]`, whose top `n` limbs are below the divisor; only the first window reaches
  // the extra limb, which that step clears.
  let mut j = N - n + 1;
  while j > 0 {
    j -= 1;
    // D3: estimate the quotient limb, which is below `2^64`, from the window's top two limbs, and
    // correct the estimate with the next divisor limb. The top limb is at most `high`. If it
    // equals `high`, the quotient of the top two limbs by `high` is at least `2^64`, so the
    // estimate starts at `2^64 - 1`, and `rest`, the remainder that goes with it, at
    // `rem[j + n - 1] + high`, which may carry out of the limb. Once `rest` reaches `2^64`, the
    // product test cannot succeed.
    let top = rem.get(j + n);
    let (mut estimate, mut rest, mut rest_overflowed) = if top == high {
      let (rest, overflowed) = rem.get(j + n - 1).overflowing_add(high);
      (u64::MAX, rest, overflowed)
    } else {
      let (estimate, rest) = div_rem_wide(top, rem.get(j + n - 1), high);
      (estimate, rest, false)
    };
    // Corrects while `estimate * next > rest * 2^64 + rem[j + n - 2]`, compared limb by limb.
    while !rest_overflowed {
      let (product_low, product_high) = carrying_mul_add(estimate, next, 0, 0);
      if product_high < rest || (product_high == rest && product_low <= rem.get(j + n - 2)) {
        break;
      }
      estimate -= 1;
      (rest, rest_overflowed) = rest.overflowing_add(high);
    }

    // D4: subtract `estimate * divisor` from the window.
    let mut carry: u64 = 0;
    let mut borrow = false;
    let mut i = 0;
    while i < n {
      let (product_low, product_high) = carrying_mul_add(estimate, divisor[i], carry, 0);
      carry = product_high;
      let (limb, first) = rem.get(j + i).overflowing_sub(product_low);
      let (limb, second) = limb.overflowing_sub(borrow as u64);
      rem.set(j + i, limb);
      borrow = first || second;
      i += 1;
    }
    let (limb, first) = rem.get(j + n).overflowing_sub(carry);
    let (limb, second) = limb.overflowing_sub(borrow as u64);
    rem.set(j + n, limb);

    // D5, D6: a borrow out of the window means that the estimate was one too large, which the
    // divisor added back corrects. The carry out of the top limb cancels the borrow.
    if first || second {
      estimate -= 1;
      let mut carry = false;
      let mut i = 0;
      while i < n {
        let (limb, first) = rem.get(j + i).overflowing_add(divisor[i]);
        let (limb, second) = limb.overflowing_add(carry as u64);
        rem.set(j + i, limb);
        carry = first || second;
        i += 1;
      }
      rem.set(j + n, rem.get(j + n).wrapping_add(carry as u64));
    }
    quotient[j] = estimate;
  }
  debug_assert!(rem.top == 0, "the remainder exceeds the divisor");
  // D8: unnormalize the remainder, which is below the normalized divisor.
  (quotient, shr(&rem.limbs, shift))
}

/// Returns `true` if `mode` rounds a quotient one step away from zero, given the magnitudes of the
/// quotient and the remainder of a division rounded toward zero and of the divisor, and whether
/// the dividend is negative. Requires `remainder < divisor`.
pub(crate) const fn rounds_away<const N: usize>(
  quotient: &[u64; N],
  remainder: &[u64; N],
  divisor: &[u64; N],
  negative: bool,
  mode: RoundingMode,
) -> bool {
  // `remainder < divisor`, so nothing borrows.
  let (rest, _) = overflowing_sub(divisor, remainder);
  rounding::increments(
    mode,
    negative,
    is_zero(remainder),
    cmp(remainder, &rest),
    quotient[0] % 2 == 1,
  )
}

/// `value / divisor` rounded to an integer with `mode`, the remainder of the division rounded
/// toward zero, and whether `mode` rounded the quotient away from zero. Requires `divisor != 0`.
pub(crate) const fn round_div<const N: usize>(
  value: &[u64; N],
  divisor: &[u64; N],
  mode: RoundingMode,
) -> ([u64; N], [u64; N], bool) {
  let (quotient, remainder) = div_rem(value, divisor);
  let away = rounds_away(&quotient, &remainder, divisor, false, mode);
  // Rounding away needs a nonzero remainder, so the divisor is at least 2 and the quotient at most
  // half of `value`: adding one does not overflow.
  let rounded = if away {
    overflowing_add(&quotient, &one()).0
  } else {
    quotient
  };
  (rounded, remainder, away)
}

/// Writes the decimal digits of `value` to the end of `buf`, without leading zeros (zero is a
/// single `0`), and returns the index of the first digit. Requires `value` to have at most
/// `DIGIT_BUF_LEN` digits, which holds for `N <= 8`.
#[inline]
pub(crate) const fn write_digits<const N: usize>(value: &[u64; N], buf: &mut DigitBuf) -> usize {
  write_digits_into(value, buf)
}

/// Writes the decimal digits of `value` to the end of `buf`, without leading zeros (zero is a
/// single `0`), and returns the index of the first digit. Requires `buf` to hold every digit of
/// `value`, which `20 * N` bytes do.
pub(crate) const fn write_digits_into<const N: usize>(value: &[u64; N], buf: &mut [u8]) -> usize {
  let mut value = *value;
  let mut start = buf.len();
  // Divides off 19 digits at a time, least significant first.
  loop {
    let (quotient, mut chunk) = div_rem_limb(&value, TEN_POW_19);
    value = quotient;
    if is_zero(&value) {
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
const fn append<const N: usize>(value: &[u64; N], scale: u64, chunk: u64) -> [u64; N] {
  let (appended, carry) = mul_add_limb(value, scale, chunk);
  debug_assert!(carry == 0, "appending digits overflowed");
  appended
}

/// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for each
/// digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires a
/// representable result.
pub(crate) const fn push_digits<const N: usize>(
  value: &[u64; N],
  bytes: &[u8],
  start: usize,
  end: usize,
) -> [u64; N] {
  let mut value = *value;
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
        value = append(&value, scale, chunk);
        chunk = 0;
        scale = 1;
      }
    }
    i += 1;
  }
  append(&value, scale, chunk)
}

/// Returns `true` if `value < 2^bits`: every bit at index `bits` or above is zero.
pub(crate) const fn fits_bits<const N: usize>(value: &[u64; N], bits: u32) -> bool {
  let mut i = (bits / 64) as usize;
  if i >= N {
    return true;
  }
  // In the limb at index `bits / 64`, only the bits below index `bits` may be set, and in the limbs
  // above it, none.
  if value[i] >> (bits % 64) != 0 {
    return false;
  }
  i += 1;
  while i < N {
    if value[i] != 0 {
      return false;
    }
    i += 1;
  }
  true
}

/// `2^bits - 1`, the largest value of `bits` bits. Requires `bits <= 64N`.
pub(crate) const fn max_of_bits<const N: usize>(bits: u32) -> [u64; N] {
  let mut max = [0; N];
  let full = (bits / 64) as usize;
  let mut i = 0;
  while i < full {
    max[i] = u64::MAX;
    i += 1;
  }
  if full < N {
    // The shift is below 64, so it does not overflow, and a shift of zero leaves the limb zero.
    max[full] = (1 << (bits % 64)) - 1;
  }
  max
}

/// Zero-extends or truncates `value` to `M` limbs.
pub(crate) const fn resize<const N: usize, const M: usize>(value: &[u64; N]) -> [u64; M] {
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
  value: &[u64; N],
) -> Option<[u64; M]> {
  let resized = resize::<N, M>(value);
  // Extending back restores every limb exactly when no dropped limb was nonzero.
  if eq(&resize::<M, N>(&resized), value) {
    Some(resized)
  } else {
    None
  }
}

#[cfg(test)]
mod tests;
