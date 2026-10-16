use core::{ops::Neg, str::FromStr};

use typenum::NonZero;

use super::{Decimal, ParseError, parse};
use crate::{int, layout::Layout};

macro_rules! native_decimal {
  ($($repr:ident: $sign:ident),+ $(,)?) => {$(
    native_decimal!(@$sign $repr);

    impl<I, F> Decimal<I, F, $repr>
    where
      I: NonZero,
      (I, F): Layout<$repr>,
    {
      /// One. Defined only when `Integral` is nonzero, because one is out of range otherwise.
      pub const ONE: Self = Self::from_coeff(Self::SCALE);
    }

    impl<I, F> Default for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      /// Returns zero.
      #[inline]
      fn default() -> Self {
        Self::ZERO
      }
    }

    impl<I, F> FromStr for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      type Err = ParseError;

      /// Parses a decimal from text; see [`parse`](Self::parse).
      #[inline]
      fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::parse(s)
      }
    }
  )+};

  (@signed $repr:ident) => {
    impl<I, F> Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      /// The largest coefficient, `10^(Integral + Fraction) - 1`.
      const MAX_COEFF: $repr = int::$repr::max_magnitude(Self::DIGITS);

      /// The smallest coefficient, `-MAX_COEFF`.
      const MIN_COEFF: $repr = int::$repr::neg(Self::MAX_COEFF);

      /// The coefficient of one, `10^Fraction`.
      const SCALE: $repr = int::$repr::pow10(Self::FRACTION);

      /// Zero.
      pub const ZERO: Self = Self::from_coeff(int::$repr::ZERO);

      /// The largest value, `10^Integral - 10^-Fraction`: every digit is `9`.
      pub const MAX: Self = Self::from_coeff(Self::MAX_COEFF);

      /// The smallest value, `-MAX`.
      pub const MIN: Self = Self::from_coeff(Self::MIN_COEFF);

      const fn in_range(coeff: $repr) -> bool {
        int::$repr::le(Self::MIN_COEFF, coeff) && int::$repr::le(coeff, Self::MAX_COEFF)
      }

      /// Returns `true` if the value is zero.
      #[must_use]
      #[inline]
      pub const fn is_zero(self) -> bool {
        int::$repr::is_zero(self.repr)
      }

      /// Returns `true` if the value is less than zero.
      #[must_use]
      #[inline]
      pub const fn is_negative(self) -> bool {
        int::$repr::is_negative(self.repr)
      }

      /// Returns `true` if the value is greater than zero.
      #[must_use]
      #[inline]
      pub const fn is_positive(self) -> bool {
        int::$repr::is_positive(self.repr)
      }

      /// Returns the coefficient: the stored integer `c` such that the value is
      /// `c / 10^Fraction`.
      #[must_use]
      #[inline]
      pub const fn repr(self) -> $repr {
        self.repr
      }

      /// Creates a value from its coefficient `repr`, meaning `repr / 10^Fraction`.
      ///
      /// Returns `None` if `|repr| > 10^(Integral + Fraction) - 1`.
      #[must_use]
      #[inline]
      pub const fn from_repr(repr: $repr) -> Option<Self> {
        if Self::in_range(repr) {
          Some(Self::from_coeff(repr))
        } else {
          None
        }
      }

      /// Parses a decimal from text.
      ///
      /// The input must match `["-"] 1*DIGIT ["." 1*DIGIT]` in ASCII: there is no `+`, whitespace,
      /// exponent or `_`, and neither the integral part (`.5`) nor the fraction part after a
      /// point (`1.`) may be missing. Leading zeros in the integral part are accepted. Fraction
      /// digits beyond `Fraction` are accepted only when they are all `0`. `-0` parses as zero.
      ///
      /// Runs in linear time and constant space.
      ///
      /// # Errors
      ///
      /// The first condition that applies, in this order, determines the error kind:
      ///
      /// 1. [`Empty`](crate::ParseErrorKind::Empty): the input is empty.
      /// 2. [`InvalidSyntax`](crate::ParseErrorKind::InvalidSyntax): the input does not match
      ///    the grammar.
      /// 3. [`OutOfRange`](crate::ParseErrorKind::OutOfRange): the integral part, without leading
      ///    zeros, has more than `Integral` digits.
      /// 4. [`Inexact`](crate::ParseErrorKind::Inexact): a fraction digit beyond the first
      ///    `Fraction` is not `0`.
      pub const fn parse(s: &str) -> Result<Self, ParseError> {
        let input = s.as_bytes();
        let digits = match parse::scan(input, Self::INTEGRAL, Self::FRACTION, true) {
          Ok(digits) => digits,
          Err(err) => return Err(err),
        };
        let mut coeff = int::$repr::push_digits(
          int::$repr::ZERO,
          input,
          digits.integral_start,
          digits.integral_end,
        );
        coeff = int::$repr::push_digits(coeff, input, digits.fraction_start, digits.fraction_end);
        coeff = int::$repr::mul_pow10(coeff, digits.padding);
        if digits.negative {
          coeff = int::$repr::neg(coeff);
        }
        Ok(Self::from_coeff(coeff))
      }

      /// Adds two values, returning `None` if the sum is outside the range of the type.
      #[must_use]
      #[inline]
      pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match int::$repr::checked_add(self.repr, rhs.repr) {
          Some(sum) if Self::in_range(sum) => Some(Self::from_coeff(sum)),
          _ => None,
        }
      }

      /// Subtracts `rhs` from `self`, returning `None` if the difference is outside the range of
      /// the type.
      #[must_use]
      #[inline]
      pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match int::$repr::checked_sub(self.repr, rhs.repr) {
          Some(difference) if Self::in_range(difference) => Some(Self::from_coeff(difference)),
          _ => None,
        }
      }

      /// Returns `-self`. Never fails, because the range is symmetric.
      #[must_use]
      #[inline]
      pub const fn neg(self) -> Self {
        Self::from_coeff(int::$repr::neg(self.repr))
      }

      /// Returns the absolute value. Never fails, because the range is symmetric.
      #[must_use]
      #[inline]
      pub const fn abs(self) -> Self {
        Self::from_coeff(int::$repr::abs(self.repr))
      }
    }

    impl<I, F> Neg for Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      type Output = Self;

      /// Returns `-self`. Never fails, because the range is symmetric.
      #[inline]
      fn neg(self) -> Self {
        Self::neg(self)
      }
    }
  };

  (@unsigned $repr:ident) => {
    impl<I, F> Decimal<I, F, $repr>
    where
      (I, F): Layout<$repr>,
    {
      /// The largest coefficient, `10^(Integral + Fraction) - 1`.
      const MAX_COEFF: $repr = int::$repr::max_magnitude(Self::DIGITS);

      /// The coefficient of one, `10^Fraction`.
      const SCALE: $repr = int::$repr::pow10(Self::FRACTION);

      /// Zero.
      pub const ZERO: Self = Self::from_coeff(int::$repr::ZERO);

      /// The largest value, `10^Integral - 10^-Fraction`: every digit is `9`.
      pub const MAX: Self = Self::from_coeff(Self::MAX_COEFF);

      /// The smallest value, zero.
      pub const MIN: Self = Self::ZERO;

      const fn in_range(coeff: $repr) -> bool {
        int::$repr::le(coeff, Self::MAX_COEFF)
      }

      /// Returns `true` if the value is zero.
      #[must_use]
      #[inline]
      pub const fn is_zero(self) -> bool {
        int::$repr::is_zero(self.repr)
      }

      /// Returns `true` if the value is greater than zero, that is, not zero.
      #[must_use]
      #[inline]
      pub const fn is_positive(self) -> bool {
        int::$repr::is_positive(self.repr)
      }

      /// Returns the coefficient: the stored integer `c` such that the value is
      /// `c / 10^Fraction`.
      #[must_use]
      #[inline]
      pub const fn repr(self) -> $repr {
        self.repr
      }

      /// Creates a value from its coefficient `repr`, meaning `repr / 10^Fraction`.
      ///
      /// Returns `None` if `repr > 10^(Integral + Fraction) - 1`.
      #[must_use]
      #[inline]
      pub const fn from_repr(repr: $repr) -> Option<Self> {
        if Self::in_range(repr) {
          Some(Self::from_coeff(repr))
        } else {
          None
        }
      }

      /// Parses a decimal from text.
      ///
      /// The input must match `1*DIGIT ["." 1*DIGIT]` in ASCII: there is no sign, whitespace,
      /// exponent or `_`, and neither the integral part (`.5`) nor the fraction part after a
      /// point (`1.`) may be missing. As for the unsigned integers of the standard library, a `-`
      /// is rejected, even in `-0`. Leading zeros in the integral part are accepted. Fraction
      /// digits beyond `Fraction` are accepted only when they are all `0`.
      ///
      /// Runs in linear time and constant space.
      ///
      /// # Errors
      ///
      /// The first condition that applies, in this order, determines the error kind:
      ///
      /// 1. [`Empty`](crate::ParseErrorKind::Empty): the input is empty.
      /// 2. [`InvalidSyntax`](crate::ParseErrorKind::InvalidSyntax): the input does not match
      ///    the grammar, which includes every input that starts with `-`.
      /// 3. [`OutOfRange`](crate::ParseErrorKind::OutOfRange): the integral part, without leading
      ///    zeros, has more than `Integral` digits.
      /// 4. [`Inexact`](crate::ParseErrorKind::Inexact): a fraction digit beyond the first
      ///    `Fraction` is not `0`.
      pub const fn parse(s: &str) -> Result<Self, ParseError> {
        let input = s.as_bytes();
        let digits = match parse::scan(input, Self::INTEGRAL, Self::FRACTION, false) {
          Ok(digits) => digits,
          Err(err) => return Err(err),
        };
        let mut coeff = int::$repr::push_digits(
          int::$repr::ZERO,
          input,
          digits.integral_start,
          digits.integral_end,
        );
        coeff = int::$repr::push_digits(coeff, input, digits.fraction_start, digits.fraction_end);
        coeff = int::$repr::mul_pow10(coeff, digits.padding);
        Ok(Self::from_coeff(coeff))
      }

      /// Adds two values, returning `None` if the sum is outside the range of the type.
      #[must_use]
      #[inline]
      pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match int::$repr::checked_add(self.repr, rhs.repr) {
          Some(sum) if Self::in_range(sum) => Some(Self::from_coeff(sum)),
          _ => None,
        }
      }

      /// Subtracts `rhs` from `self`, returning `None` if the difference is negative.
      #[must_use]
      #[inline]
      pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        // A difference that is not negative is at most `self`, so it is in range.
        match int::$repr::checked_sub(self.repr, rhs.repr) {
          Some(difference) => Some(Self::from_coeff(difference)),
          None => None,
        }
      }
    }
  };
}

native_decimal!(
  i8: signed,
  i16: signed,
  i32: signed,
  i64: signed,
  i128: signed,
  u8: unsigned,
  u16: unsigned,
  u32: unsigned,
  u64: unsigned,
  u128: unsigned,
);
