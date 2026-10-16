//! The integer core.
//!
//! `Decimal` performs every operation on its coefficient through this module. The core has two
//! layers. The lower layer, a module for each of `u8` through `u128`, works on magnitudes: it reads
//! and writes decimal digits, computes powers of ten and scales by them, and adds and subtracts
//! with overflow checks. The upper layer, a module for each of `i8` through `i128`, handles the
//! sign and passes magnitude work to the unsigned module of the same width through `unsigned_abs`.
//! A function whose native implementation does not depend on the sign has the same implementation
//! in both layers.
//!
//! Every module provides `const fn`s with the same names and contracts, so storage-specific code
//! calls them in constant contexts, and storage-generic code reaches them through the sealed
//! storage trait. Storage-generic code computes in 128 bits, so the functions that only it uses
//! exist only for `u128` and `i128`.
//!
//! Preconditions are stated per function. Callers establish them from the coefficient bound
//! `|c| <= 10^digits - 1` that every `Decimal` maintains. A violated precondition is a bug in this
//! crate: arithmetic then overflows as native arithmetic does, and narrowing truncates.

/// Length of a buffer that holds the decimal digits of any native magnitude; `u128::MAX` has 39.
pub(crate) const DIGIT_BUF_LEN: usize = 39;

/// Decimal digits of a magnitude, written right-aligned by `write_digits`.
// The modules below shadow the primitive type names in this module.
pub(crate) type DigitBuf = [core::primitive::u8; DIGIT_BUF_LEN];

macro_rules! native {
  (unsigned $uint:ident { $($extra:item)* }) => {
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

      /// Converts `value` to `i128`. Requires `value <= i128::MAX`, which holds for every value
      /// narrower than 128 bits and, for `u128`, for every coefficient, because coefficients are
      /// at most `10^38 - 1`.
      #[inline]
      pub(crate) const fn to_i128(value: $uint) -> i128 {
        value as i128
      }

      $($extra)*
    }
  };
  (signed $int:ident => $uint:ident { $($extra:item)* }) => {
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

      /// Widens `value` to `i128`.
      #[inline]
      pub(crate) const fn to_i128(value: $int) -> i128 {
        value as i128
      }

      $($extra)*
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

    /// Appends the ASCII digits `digits[start..end]` to `value`, computing `value * 10 + d` for
    /// each digit `d`. Requires ASCII digits and a representable result.
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

    /// Narrows `value` from `i128`. Requires `value` to be representable, so it is not negative
    /// for an unsigned type.
    #[inline]
    pub(crate) const fn from_i128(value: i128) -> $int {
      value as $int
    }
  };
}

native!(unsigned u8 {});
native!(unsigned u16 {});
native!(unsigned u32 {});
native!(unsigned u64 {});
native!(unsigned u128 {
  /// `value * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
  /// representable.
  #[inline]
  pub(crate) const fn checked_mul_pow10(value: u128, exp: u32) -> Option<u128> {
    value.checked_mul(pow10(exp))
  }

  /// Quotient and remainder of `value / 10^exp`. Requires `10^exp` to be representable.
  #[inline]
  pub(crate) const fn div_rem_pow10(value: u128, exp: u32) -> (u128, u128) {
    let divisor = pow10(exp);
    (value / divisor, value % divisor)
  }
});

native!(signed i8 => u8 {});
native!(signed i16 => u16 {});
native!(signed i32 => u32 {});
native!(signed i64 => u64 {});
native!(signed i128 => u128 {
  /// `lhs * rhs`. Requires the product to be representable.
  #[inline]
  pub(crate) const fn mul(lhs: i128, rhs: i128) -> i128 {
    lhs * rhs
  }

  /// `value * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
  /// representable.
  #[inline]
  pub(crate) const fn checked_mul_pow10(value: i128, exp: u32) -> Option<i128> {
    match super::u128::checked_mul_pow10(value.unsigned_abs(), exp) {
      Some(magnitude) if is_negative(value) => 0i128.checked_sub_unsigned(magnitude),
      Some(magnitude) => 0i128.checked_add_unsigned(magnitude),
      None => None,
    }
  }

  /// Quotient and remainder of `value / 10^exp`, rounding toward zero. As with `/` and `%`, both
  /// have the sign of `value`. Requires `10^exp` to be representable.
  #[inline]
  pub(crate) const fn div_rem_pow10(value: i128, exp: u32) -> (i128, i128) {
    let (quotient, remainder) = super::u128::div_rem_pow10(value.unsigned_abs(), exp);
    // Both magnitudes are at most `|value|`, so each is exact with the sign of `value`.
    if is_negative(value) {
      (
        0i128.wrapping_sub_unsigned(quotient),
        0i128.wrapping_sub_unsigned(remainder),
      )
    } else {
      (quotient as i128, remainder as i128)
    }
  }
});

#[cfg(test)]
mod tests;
