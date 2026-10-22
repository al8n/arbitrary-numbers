use core::{ops::Neg, str::FromStr};

use typenum::NonZero;

use super::{Decimal, ParseError, parse};
use crate::{
  i192, i256, i320, i384, i448, i512, int, layout::Layout, u192, u256, u320, u384, u448, u512,
};

/// Implements the constants, `const fn` methods and traits that depend only on the storage, for
/// each listed storage with its sign, `signed` or `unsigned`, and its kind, `native` or `wide`.
/// The kind selects how the examples import the storage and spell its coefficients.
macro_rules! storage_decimal {
  ($($repr:ident: $sign:ident $kind:ident),+ $(,)?) => {$(
    storage_decimal!(@$sign $repr $kind);

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

  (@signed $repr:ident $kind:ident) => {
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

      /// Parses a decimal from text, exactly.
      ///
      /// The input is ASCII text in the following grammar, in which letters match in either case.
      /// It accepts the numeric strings of the General Decimal Arithmetic specification, every
      /// form that [`f64`] parses with [`FromStr`], and `_` separators as in Rust literals.
      ///
      /// ```text
      /// number     = [sign] (finite / nonfinite)
      /// sign       = "+" / "-"
      /// finite     = (digits ["." [digits]] / "." digits) [exponent]
      /// exponent   = ("e" / "E") [sign] exp-digits
      /// digits     = DIGIT *(DIGIT / "_")
      /// exp-digits = *"_" DIGIT *(DIGIT / "_")
      /// nonfinite  = "inf" / "infinity" / ["s"] "nan" *DIGIT
      /// ```
      ///
      /// The integral and fraction digits each start with a digit, after which `_` may appear
      /// anywhere, also repeated or at the end. The exponent's digits may have `_` anywhere after
      /// its sign, also first, but need at least one digit. So `1_000.000_1`, `1__0`, `5_.5`,
      /// `1e_1_0`, `1e+_5`, `+.5`, `5.` and `.5e3` are valid, while `_1`, `._5`, `1._5`, `1e_`,
      /// `1e_+5`, `.`, `+` and `e5` are not. Whitespace, `,`, hexadecimal and non-ASCII digits are
      /// never accepted.
      ///
      /// The value is the mantissa times ten to the power of the exponent, and it must fit the
      /// type exactly: once the exponent has moved the point, the integral part may have at most
      /// `Integral` digits after its leading zeros, and the fraction digits beyond the first
      /// `Fraction` must all be `0`. Nothing is rounded. The exponent may have any number of
      /// digits and is applied exactly, so `1000e-3` and `0.0001e4` are both one, and zero parses
      /// with any exponent, as in `0e999999999`. `-0` parses as zero. A decimal is always finite,
      /// so an infinity or a NaN, including a signaling `sNaN` and a NaN with diagnostic digits
      /// such as `NaN123`, is recognized but reported as an error.
      ///
      /// Runs in linear time and constant space.
      ///
      /// # Errors
      ///
      /// The first condition that applies, in this order, determines the error kind:
      ///
      /// 1. [`Empty`](crate::ParseErrorKind::Empty): the input is empty. Whitespace alone is
      ///    invalid syntax.
      /// 2. [`InvalidSyntax`](crate::ParseErrorKind::InvalidSyntax): the input does not match
      ///    the grammar. The whole input is checked before its value, so `1e999_x` is invalid
      ///    syntax rather than out of range.
      /// 3. [`NotFinite`](crate::ParseErrorKind::NotFinite): the input is an infinity or a NaN.
      /// 4. [`OutOfRange`](crate::ParseErrorKind::OutOfRange): once the exponent has moved the
      ///    point, the integral part has more than `Integral` digits after its leading zeros.
      /// 5. [`Inexact`](crate::ParseErrorKind::Inexact): once the exponent has moved the point,
      ///    a fraction digit beyond the first `Fraction` is not `0`.
      ///
      /// # Examples
      ///
      /// ```
      #[doc = storage_decimal!(@use signed $kind $repr)]
      ///
      #[doc = concat!("type D = Decimal<U1, U1, ", stringify!($repr), ">;")]
      ///
      /// for text in ["2.5", "+2.5", "02.50", "25e-1", "0.25E+1", ".25e1", "2_5e-1", "250_000e-5"] {
      #[doc = concat!("  assert_eq!(D::parse(text)?", storage_decimal!(@coeff $kind "25"), ");")]
      /// }
      #[doc = concat!("assert_eq!(D::parse(\"-5.\")?", storage_decimal!(@coeff $kind "-50"), ");")]
      /// assert_eq!(D::parse("-0e5")?, D::ZERO);
      ///
      /// let kind = |text| D::parse(text).map_err(|err| err.kind());
      /// assert_eq!(kind(""), Err(ParseErrorKind::Empty));
      /// assert_eq!(kind(" 1"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("1e9_x"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("-Infinity"), Err(ParseErrorKind::NotFinite));
      /// assert_eq!(kind("NaN"), Err(ParseErrorKind::NotFinite));
      /// assert_eq!(kind("1e1"), Err(ParseErrorKind::OutOfRange));
      /// assert_eq!(kind("0.05"), Err(ParseErrorKind::Inexact));
      /// // Out of range and inexact at once.
      /// assert_eq!(kind("10.05"), Err(ParseErrorKind::OutOfRange));
      /// # Ok::<(), numo::ParseError>(())
      /// ```
      pub const fn parse(s: &str) -> Result<Self, ParseError> {
        let input = s.as_bytes();
        let digits = match parse::scan(input, Self::INTEGRAL, Self::FRACTION, true) {
          Ok(digits) => digits,
          Err(err) => return Err(err),
        };
        let mut coeff = int::$repr::push_digits(int::$repr::ZERO, input, digits.start, digits.end);
        coeff = int::$repr::mul_pow10(coeff, digits.scale);
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

  (@unsigned $repr:ident $kind:ident) => {
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

      /// Parses a decimal from text, exactly.
      ///
      /// The input is ASCII text in the following grammar, in which letters match in either case.
      /// It accepts the numeric strings of the General Decimal Arithmetic specification, every
      /// form that [`f64`] parses with [`FromStr`], and `_` separators as in Rust literals, except
      /// that the number has no `-` sign: as for the unsigned integers of the standard library, a
      /// leading `-` is rejected, even in `-0` or `-inf`. The sign of the exponent may be `-`.
      ///
      /// ```text
      /// number     = ["+"] (finite / nonfinite)
      /// finite     = (digits ["." [digits]] / "." digits) [exponent]
      /// exponent   = ("e" / "E") [sign] exp-digits
      /// sign       = "+" / "-"
      /// digits     = DIGIT *(DIGIT / "_")
      /// exp-digits = *"_" DIGIT *(DIGIT / "_")
      /// nonfinite  = "inf" / "infinity" / ["s"] "nan" *DIGIT
      /// ```
      ///
      /// The integral and fraction digits each start with a digit, after which `_` may appear
      /// anywhere, also repeated or at the end. The exponent's digits may have `_` anywhere after
      /// its sign, also first, but need at least one digit. So `1_000.000_1`, `1__0`, `5_.5`,
      /// `1e_1_0`, `1e+_5`, `+.5`, `5.` and `.5e3` are valid, while `_1`, `._5`, `1._5`, `1e_`,
      /// `1e_+5`, `.`, `+` and `e5` are not. Whitespace, `,`, hexadecimal and non-ASCII digits are
      /// never accepted.
      ///
      /// The value is the mantissa times ten to the power of the exponent, and it must fit the
      /// type exactly: once the exponent has moved the point, the integral part may have at most
      /// `Integral` digits after its leading zeros, and the fraction digits beyond the first
      /// `Fraction` must all be `0`. Nothing is rounded. The exponent may have any number of
      /// digits and is applied exactly, so `1000e-3` and `0.0001e4` are both one, and zero parses
      /// with any exponent, as in `0e999999999`. A decimal is always finite, so an infinity or a
      /// NaN, including a signaling `sNaN` and a NaN with diagnostic digits such as `NaN123`, is
      /// recognized but reported as an error.
      ///
      /// Runs in linear time and constant space.
      ///
      /// # Errors
      ///
      /// The first condition that applies, in this order, determines the error kind:
      ///
      /// 1. [`Empty`](crate::ParseErrorKind::Empty): the input is empty. Whitespace alone is
      ///    invalid syntax.
      /// 2. [`InvalidSyntax`](crate::ParseErrorKind::InvalidSyntax): the input does not match
      ///    the grammar, which includes every input that starts with `-`. The whole input is
      ///    checked before its value, so `1e999_x` is invalid syntax rather than out of range.
      /// 3. [`NotFinite`](crate::ParseErrorKind::NotFinite): the input is an infinity or a NaN.
      /// 4. [`OutOfRange`](crate::ParseErrorKind::OutOfRange): once the exponent has moved the
      ///    point, the integral part has more than `Integral` digits after its leading zeros.
      /// 5. [`Inexact`](crate::ParseErrorKind::Inexact): once the exponent has moved the point,
      ///    a fraction digit beyond the first `Fraction` is not `0`.
      ///
      /// # Examples
      ///
      /// ```
      #[doc = storage_decimal!(@use unsigned $kind $repr)]
      ///
      #[doc = concat!("type D = Decimal<U1, U1, ", stringify!($repr), ">;")]
      ///
      /// for text in ["2.5", "+2.5", "02.50", "25e-1", "0.25E+1", ".25e1", "2_5e-1", "250_000e-5"] {
      #[doc = concat!("  assert_eq!(D::parse(text)?", storage_decimal!(@coeff $kind "25"), ");")]
      /// }
      /// assert_eq!(D::parse("+0e5")?, D::ZERO);
      ///
      /// let kind = |text| D::parse(text).map_err(|err| err.kind());
      /// assert_eq!(kind(""), Err(ParseErrorKind::Empty));
      /// assert_eq!(kind("-0"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("-inf"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("1e9_x"), Err(ParseErrorKind::InvalidSyntax));
      /// assert_eq!(kind("inf"), Err(ParseErrorKind::NotFinite));
      /// assert_eq!(kind("1e1"), Err(ParseErrorKind::OutOfRange));
      /// assert_eq!(kind("0.05"), Err(ParseErrorKind::Inexact));
      /// // Out of range and inexact at once.
      /// assert_eq!(kind("10.05"), Err(ParseErrorKind::OutOfRange));
      /// # Ok::<(), numo::ParseError>(())
      /// ```
      pub const fn parse(s: &str) -> Result<Self, ParseError> {
        let input = s.as_bytes();
        let digits = match parse::scan(input, Self::INTEGRAL, Self::FRACTION, false) {
          Ok(digits) => digits,
          Err(err) => return Err(err),
        };
        let coeff = int::$repr::push_digits(int::$repr::ZERO, input, digits.start, digits.end);
        Ok(Self::from_coeff(int::$repr::mul_pow10(coeff, digits.scale)))
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

  // The imports of an example, in `rustfmt` order.
  (@use $sign:ident native $repr:ident) => {
    "use numo::{Decimal, ParseErrorKind, typenum::U1};"
  };
  (@use signed wide $repr:ident) => {
    concat!(
      "use numo::{Decimal, ParseErrorKind, ",
      stringify!($repr),
      ", typenum::U1};"
    )
  };
  (@use unsigned wide $repr:ident) => {
    concat!(
      "use numo::{Decimal, ParseErrorKind, typenum::U1, ",
      stringify!($repr),
      "};"
    )
  };

  // A native coefficient compares with an integer literal, a wide one through its text.
  (@coeff native $coeff:literal) => {
    concat!(".repr(), ", $coeff)
  };
  (@coeff wide $coeff:literal) => {
    concat!(".repr().to_string(), \"", $coeff, "\"")
  };
}

storage_decimal!(
  i8: signed native,
  i16: signed native,
  i32: signed native,
  i64: signed native,
  i128: signed native,
  i192: signed wide,
  i256: signed wide,
  i320: signed wide,
  i384: signed wide,
  i448: signed wide,
  i512: signed wide,
  u8: unsigned native,
  u16: unsigned native,
  u32: unsigned native,
  u64: unsigned native,
  u128: unsigned native,
  u192: unsigned wide,
  u256: unsigned wide,
  u320: unsigned wide,
  u384: unsigned wide,
  u448: unsigned wide,
  u512: unsigned wide,
);
