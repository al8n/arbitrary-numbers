//! The integer core.
//!
//! `Decimal` performs every operation on its coefficient through this module. The core has two
//! layers. The lower layer works on magnitudes: it reads and writes decimal digits, computes powers
//! of ten and scales by them, multiplies, and adds and subtracts with overflow checks. The upper
//! layer handles the sign and passes magnitude work to the lower layer through `unsigned_abs`. A
//! function whose implementation does not depend on the sign has the same implementation in both
//! layers.
//!
//! Each storage has a module of the same name. For the native storages, the modules for `u8`
//! through `u128` form the lower layer and the modules for `i8` through `i128` the upper layer. The
//! wide storages `u192` through `u320` and `i192` through `i320` hold arrays of 64-bit limbs, least
//! significant first. Their modules apply `limbs`, the lower layer for `[u64; N]`, and
//! `signed_limbs`, the upper layer, which reads the array as two's complement.
//!
//! Every storage module provides `const fn`s with the same names and contracts, so storage-specific
//! code calls them in constant contexts, and storage-generic code reaches them through the sealed
//! storage trait. Each module also converts its values to and from limbs (two limbs for a native
//! storage), which conversions between storages of different widths go through.
//!
//! Preconditions are stated per function. Callers establish them from the coefficient bound
//! `|c| <= 10^digits - 1` that every `Decimal` maintains. A violated precondition is a bug in this
//! crate: native arithmetic then overflows as native arithmetic does, multi-limb arithmetic wraps
//! and fails a debug assertion, and narrowing truncates.

/// Length of a buffer that holds the decimal digits of any magnitude: `u320::MAX` and `|i320::MIN|`
/// have 97.
pub(crate) const DIGIT_BUF_LEN: usize = 97;

/// Decimal digits of a magnitude, written right-aligned by `write_digits`.
// The modules below shadow the primitive type names in this module.
pub(crate) type DigitBuf = [core::primitive::u8; DIGIT_BUF_LEN];

pub(crate) mod limbs;
pub(crate) mod signed_limbs;

macro_rules! native {
  (unsigned $uint:ident) => {
    #[doc = concat!("Lower layer for `", stringify!($uint), "`.")]
    pub(crate) mod $uint {
      use super::{DIGIT_BUF_LEN, DigitBuf};

      native!(@shared $uint);

      /// Writes the decimal digits of `value` to the end of `buf`, without leading zeros (zero is
      /// a single `0`), and returns the index of the first digit.
      pub(crate) const fn write_digits(mut value: $uint, buf: &mut DigitBuf) -> usize {
        let mut start = DIGIT_BUF_LEN;
        loop {
          start -= 1;
          buf[start] = b'0' + (value % 10) as u8;
          value /= 10;
          if value == 0 {
            return start;
          }
        }
      }

      /// `value * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
      /// representable.
      #[inline]
      pub(crate) const fn checked_mul_pow10(value: $uint, exp: u32) -> Option<$uint> {
        value.checked_mul(pow10(exp))
      }

      /// Quotient and remainder of `value / 10^exp`. Requires `10^exp` to be representable.
      #[inline]
      pub(crate) const fn div_rem_pow10(value: $uint, exp: u32) -> ($uint, $uint) {
        let divisor = pow10(exp);
        (value / divisor, value % divisor)
      }

      /// Zero-extends `value` to two limbs.
      #[inline]
      pub(crate) const fn to_limbs(value: $uint) -> [u64; 2] {
        let wide = value as u128;
        [wide as u64, (wide >> 64) as u64]
      }

      /// The unsigned integer in `limbs`, or `None` if it is not representable.
      #[inline]
      pub(crate) const fn checked_from_limbs(limbs: [u64; 2]) -> Option<$uint> {
        let wide = native!(@u128 limbs);
        if wide <= $uint::MAX as u128 {
          Some(wide as $uint)
        } else {
          None
        }
      }
    }
  };
  (signed $int:ident => $uint:ident) => {
    #[doc = concat!(
      "Upper layer for `",
      stringify!($int),
      "`, over the lower layer for `",
      stringify!($uint),
      "`."
    )]
    pub(crate) mod $int {
      use super::DigitBuf;

      native!(@shared $int);

      #[inline]
      pub(crate) const fn is_negative(value: $int) -> bool {
        value < 0
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

      /// Writes the decimal digits of `|value|` to the end of `buf`, without leading zeros (zero
      /// is a single `0`), and returns the index of the first digit.
      #[inline]
      pub(crate) const fn write_digits(value: $int, buf: &mut DigitBuf) -> usize {
        super::$uint::write_digits(value.unsigned_abs(), buf)
      }

      /// `value * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
      /// representable.
      #[inline]
      pub(crate) const fn checked_mul_pow10(value: $int, exp: u32) -> Option<$int> {
        match super::$uint::checked_mul_pow10(value.unsigned_abs(), exp) {
          Some(magnitude) if is_negative(value) => ZERO.checked_sub_unsigned(magnitude),
          Some(magnitude) => ZERO.checked_add_unsigned(magnitude),
          None => None,
        }
      }

      /// Quotient and remainder of `value / 10^exp`, rounding toward zero. As with `/` and `%`,
      /// both have the sign of `value`. Requires `10^exp` to be representable.
      #[inline]
      pub(crate) const fn div_rem_pow10(value: $int, exp: u32) -> ($int, $int) {
        let (quotient, remainder) = super::$uint::div_rem_pow10(value.unsigned_abs(), exp);
        // Both magnitudes are at most `|value|`, so each is exact with the sign of `value`.
        if is_negative(value) {
          (
            ZERO.wrapping_sub_unsigned(quotient),
            ZERO.wrapping_sub_unsigned(remainder),
          )
        } else {
          (quotient as $int, remainder as $int)
        }
      }

      /// Sign-extends `value` to two limbs.
      #[inline]
      pub(crate) const fn to_limbs(value: $int) -> [u64; 2] {
        let wide = value as i128;
        [wide as u64, (wide >> 64) as u64]
      }

      /// The two's complement integer in `limbs`, or `None` if it is not representable.
      #[inline]
      pub(crate) const fn checked_from_limbs(limbs: [u64; 2]) -> Option<$int> {
        let wide = native!(@u128 limbs) as i128;
        if $int::MIN as i128 <= wide && wide <= $int::MAX as i128 {
          Some(wide as $int)
        } else {
          None
        }
      }
    }
  };
  (@shared $int:ident) => {
    pub(crate) const ZERO: $int = 0;

    #[inline]
    pub(crate) const fn is_zero(value: $int) -> bool {
      value == 0
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
    #[inline]
    pub(crate) const fn mul(lhs: $int, rhs: $int) -> $int {
      lhs * rhs
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

    /// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for
    /// each digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires
    /// a representable result.
    pub(crate) const fn push_digits(
      mut value: $int,
      bytes: &[u8],
      start: usize,
      end: usize,
    ) -> $int {
      let mut i = start;
      while i < end {
        let byte = bytes[i];
        if byte.is_ascii_digit() {
          value = value * 10 + (byte - b'0') as $int;
        }
        i += 1;
      }
      value
    }

    /// Truncates `limbs`, a 128-bit integer, to this type. Requires its value, read as two's
    /// complement for a signed type, to be representable.
    #[inline]
    pub(crate) const fn from_limbs(limbs: [u64; 2]) -> $int {
      native!(@u128 limbs) as $int
    }
  };
  (@u128 $limbs:ident) => {
    $limbs[0] as u128 | ($limbs[1] as u128) << 64
  };
}

native!(unsigned u8);
native!(unsigned u16);
native!(unsigned u32);
native!(unsigned u64);
native!(unsigned u128);

native!(signed i8 => u8);
native!(signed i16 => u16);
native!(signed i32 => u32);
native!(signed i64 => u64);
native!(signed i128 => u128);

macro_rules! wide {
  (unsigned $uint:ident: $limbs:literal) => {
    #[doc = concat!(
      "Lower layer for `",
      stringify!($uint),
      "`, over `[u64; ",
      stringify!($limbs),
      "]`."
    )]
    pub(crate) mod $uint {
      use super::{DigitBuf, limbs};
      use crate::$uint;

      wide!(@shared $uint, limbs, $limbs);
    }
  };
  (signed $int:ident: $limbs:literal) => {
    #[doc = concat!(
      "Upper layer for `",
      stringify!($int),
      "`, over `[u64; ",
      stringify!($limbs),
      "]` in two's complement."
    )]
    pub(crate) mod $int {
      use super::{DigitBuf, signed_limbs};
      use crate::$int;

      wide!(@shared $int, signed_limbs, $limbs);

      #[inline]
      pub(crate) const fn is_negative(value: $int) -> bool {
        signed_limbs::is_negative(value.to_limbs())
      }

      /// `-value`. Requires `value != MIN`.
      #[inline]
      pub(crate) const fn neg(value: $int) -> $int {
        $int::from_limbs(signed_limbs::neg(value.to_limbs()))
      }

      /// `|value|`. Requires `value != MIN`.
      #[inline]
      pub(crate) const fn abs(value: $int) -> $int {
        $int::from_limbs(signed_limbs::abs(value.to_limbs()))
      }
    }
  };
  (@shared $ty:ident, $layer:ident, $limbs:literal) => {
    pub(crate) const ZERO: $ty = $ty::ZERO;

    #[inline]
    pub(crate) const fn is_zero(value: $ty) -> bool {
      $layer::is_zero(value.to_limbs())
    }

    #[inline]
    pub(crate) const fn is_positive(value: $ty) -> bool {
      $layer::is_positive(value.to_limbs())
    }

    #[inline]
    pub(crate) const fn cmp(lhs: $ty, rhs: $ty) -> core::cmp::Ordering {
      $layer::cmp(lhs.to_limbs(), rhs.to_limbs())
    }

    #[inline]
    pub(crate) const fn le(lhs: $ty, rhs: $ty) -> bool {
      $layer::le(lhs.to_limbs(), rhs.to_limbs())
    }

    #[inline]
    pub(crate) const fn checked_add(lhs: $ty, rhs: $ty) -> Option<$ty> {
      match $layer::checked_add(lhs.to_limbs(), rhs.to_limbs()) {
        Some(sum) => Some($ty::from_limbs(sum)),
        None => None,
      }
    }

    #[inline]
    pub(crate) const fn checked_sub(lhs: $ty, rhs: $ty) -> Option<$ty> {
      match $layer::checked_sub(lhs.to_limbs(), rhs.to_limbs()) {
        Some(difference) => Some($ty::from_limbs(difference)),
        None => None,
      }
    }

    /// `lhs * rhs`. Requires the product to be representable.
    #[inline]
    pub(crate) const fn mul(lhs: $ty, rhs: $ty) -> $ty {
      $ty::from_limbs($layer::mul(lhs.to_limbs(), rhs.to_limbs()))
    }

    /// `10^exp`. Requires `10^exp` to be representable.
    #[inline]
    pub(crate) const fn pow10(exp: u32) -> $ty {
      $ty::from_limbs($layer::pow10(exp))
    }

    /// `10^digits - 1`, the largest magnitude with `digits` decimal digits. Requires `10^digits`
    /// to be representable.
    #[inline]
    pub(crate) const fn max_magnitude(digits: u32) -> $ty {
      $ty::from_limbs($layer::max_magnitude(digits))
    }

    /// `value * 10^exp`. Requires the product to be representable.
    #[inline]
    pub(crate) const fn mul_pow10(value: $ty, exp: u32) -> $ty {
      $ty::from_limbs($layer::mul_pow10(value.to_limbs(), exp))
    }

    /// `value * 10^exp`, or `None` if the product is not representable.
    #[inline]
    pub(crate) const fn checked_mul_pow10(value: $ty, exp: u32) -> Option<$ty> {
      match $layer::checked_mul_pow10(value.to_limbs(), exp) {
        Some(product) => Some($ty::from_limbs(product)),
        None => None,
      }
    }

    /// Quotient and remainder of `value / 10^exp`, rounding toward zero. For a signed type, as with
    /// `/` and `%`, both have the sign of `value`.
    #[inline]
    pub(crate) const fn div_rem_pow10(value: $ty, exp: u32) -> ($ty, $ty) {
      let (quotient, remainder) = $layer::div_rem_pow10(value.to_limbs(), exp);
      ($ty::from_limbs(quotient), $ty::from_limbs(remainder))
    }

    /// Writes the decimal digits of the magnitude of `value` to the end of `buf`, without leading
    /// zeros (zero is a single `0`), and returns the index of the first digit.
    #[inline]
    pub(crate) const fn write_digits(value: $ty, buf: &mut DigitBuf) -> usize {
      $layer::write_digits(value.to_limbs(), buf)
    }

    /// Appends the ASCII digits in `bytes[start..end]` to `value`, computing `value * 10 + d` for
    /// each digit `d` and skipping every other byte, such as the separators `_` and `.`. Requires
    /// `value` not to be negative and a representable result.
    #[inline]
    pub(crate) const fn push_digits(value: $ty, bytes: &[u8], start: usize, end: usize) -> $ty {
      $ty::from_limbs($layer::push_digits(value.to_limbs(), bytes, start, end))
    }

    #[inline]
    pub(crate) const fn to_limbs(value: $ty) -> [u64; $limbs] {
      value.to_limbs()
    }

    #[inline]
    pub(crate) const fn from_limbs(value: [u64; $limbs]) -> $ty {
      $ty::from_limbs(value)
    }

    /// Every bit pattern is a value, so this always succeeds.
    #[inline]
    pub(crate) const fn checked_from_limbs(value: [u64; $limbs]) -> Option<$ty> {
      Some($ty::from_limbs(value))
    }
  };
}

wide!(unsigned u192: 3);
wide!(unsigned u256: 4);
wide!(unsigned u320: 5);

wide!(signed i192: 3);
wide!(signed i256: 4);
wide!(signed i320: 5);

#[cfg(test)]
mod tests;
