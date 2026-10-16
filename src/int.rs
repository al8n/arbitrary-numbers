//! The integer core.
//!
//! `Decimal` performs every operation on its coefficient through this module. Each native storage
//! has a submodule of `const fn`s with the same names and contracts, so storage-specific code calls
//! them in constant contexts, and storage-generic code reaches them through the sealed storage
//! trait.
//!
//! Preconditions are stated per function. Callers establish them from the coefficient bound
//! `|c| <= 10^digits - 1` that every `Decimal` maintains. A violated precondition is a bug in this
//! crate: arithmetic then overflows as native arithmetic does, and narrowing truncates.

/// Length of a buffer that holds the decimal digits of any native magnitude; `u128::MAX` has 39.
pub(crate) const DIGIT_BUF_LEN: usize = 39;

/// Decimal digits of a magnitude, written right-aligned by `write_digits`.
pub(crate) type DigitBuf = [u8; DIGIT_BUF_LEN];

macro_rules! native {
  ($($int:ident),+ $(,)?) => {$(
    #[doc = concat!("Integer core for `", stringify!($int), "`.")]
    pub(crate) mod $int {
      use super::{DIGIT_BUF_LEN, DigitBuf};

      pub(crate) const ZERO: $int = 0;

      #[inline]
      pub(crate) const fn is_zero(value: $int) -> bool {
        value == 0
      }

      #[inline]
      pub(crate) const fn is_negative(value: $int) -> bool {
        value < 0
      }

      #[inline]
      pub(crate) const fn is_positive(value: $int) -> bool {
        value > 0
      }

      #[inline]
      pub(crate) const fn le(lhs: $int, rhs: $int) -> bool {
        lhs <= rhs
      }

      #[inline]
      pub(crate) const fn checked_add(lhs: $int, rhs: $int) -> Option<$int> {
        lhs.checked_add(rhs)
      }

      #[inline]
      pub(crate) const fn checked_sub(lhs: $int, rhs: $int) -> Option<$int> {
        lhs.checked_sub(rhs)
      }

      /// `lhs * rhs`. Requires the product to be representable.
      #[allow(dead_code, reason = "storage-generic multiplication runs in the `i128` core")]
      #[inline]
      pub(crate) const fn mul(lhs: $int, rhs: $int) -> $int {
        lhs * rhs
      }

      /// `-value`. Requires `value != MIN`.
      #[inline]
      pub(crate) const fn neg(value: $int) -> $int {
        -value
      }

      /// `|value|`. Requires `value != MIN`.
      #[inline]
      pub(crate) const fn abs(value: $int) -> $int {
        value.abs()
      }

      /// `10^exp`. Requires `10^exp` to be representable.
      #[inline]
      pub(crate) const fn pow10(exp: u32) -> $int {
        (10 as $int).pow(exp)
      }

      /// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits`
      /// to be representable.
      #[inline]
      pub(crate) const fn max_magnitude(digits: u32) -> $int {
        pow10(digits) - 1
      }

      /// `value * 10^exp`. Requires the product to be representable.
      #[inline]
      pub(crate) const fn mul_pow10(value: $int, exp: u32) -> $int {
        value * pow10(exp)
      }

      /// `value * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to
      /// be representable.
      #[allow(dead_code, reason = "storage-generic conversion runs in the `i128` core")]
      #[inline]
      pub(crate) const fn checked_mul_pow10(value: $int, exp: u32) -> Option<$int> {
        value.checked_mul(pow10(exp))
      }

      /// Quotient and remainder of `value / 10^exp`, rounding toward zero; the remainder has the
      /// sign of `value`. Requires `10^exp` to be representable.
      #[allow(dead_code, reason = "storage-generic conversion runs in the `i128` core")]
      #[inline]
      pub(crate) const fn div_rem_pow10(value: $int, exp: u32) -> ($int, $int) {
        let divisor = pow10(exp);
        (value / divisor, value % divisor)
      }

      /// Appends the ASCII digits `digits[start..end]` to `value`, computing `value * 10 + d`
      /// for each digit `d`. Requires ASCII digits and a representable result.
      pub(crate) const fn push_digits(
        mut value: $int,
        digits: &[u8],
        start: usize,
        end: usize,
      ) -> $int {
        let mut i = start;
        while i < end {
          value = value * 10 + (digits[i] - b'0') as $int;
          i += 1;
        }
        value
      }

      /// Writes the decimal digits of `|value|` to the end of `buf`, without leading zeros (zero
      /// is a single `0`), and returns the index of the first digit.
      pub(crate) const fn write_digits(value: $int, buf: &mut DigitBuf) -> usize {
        let mut magnitude = value.unsigned_abs();
        let mut start = DIGIT_BUF_LEN;
        loop {
          start -= 1;
          buf[start] = b'0' + (magnitude % 10) as u8;
          magnitude /= 10;
          if magnitude == 0 {
            return start;
          }
        }
      }

      /// Widens `value` to `i128`.
      #[inline]
      pub(crate) const fn to_i128(value: $int) -> i128 {
        value as i128
      }

      /// Narrows `value` from `i128`. Requires `value` to be representable.
      #[inline]
      pub(crate) const fn from_i128(value: i128) -> $int {
        value as $int
      }
    }
  )+};
}

native!(i8, i16, i32, i64, i128);

#[cfg(test)]
mod tests;
