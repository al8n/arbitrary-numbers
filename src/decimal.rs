//! Fixed-point decimal numbers.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  marker::PhantomData,
  ops::{Add, BitAnd},
};

use typenum::{And, Sum};

pub use error::{ConvertError, ParseError, ParseErrorKind};

use crate::{
  int,
  layout::{DigitCount, Layout, Pick, SealedLayout, SealedStorage, Storage, Widen, WidenRepr},
};

mod error;
mod format;
mod native;
mod parse;

#[cfg(test)]
mod tests;

/// An exact fixed-point decimal number with at most `Integral` integer digits and exactly
/// `Fraction` fraction digits, stored as a signed or unsigned integer coefficient of type `Repr`.
///
/// The value is `c / 10^Fraction`, where the coefficient `c` is the integer stored in `Repr` and
/// `|c| <= 10^(Integral + Fraction) - 1`. With a signed `Repr`, the range is symmetric
/// ([`MIN`](Self::MIN) is `-MAX`); with an unsigned `Repr`, it runs from zero to `MAX`, and `MIN`
/// is zero. There is no NaN, infinity or negative zero. The type corresponds to SQL
/// `NUMERIC(Integral + Fraction, Fraction)`. Because the number of fraction digits is part of the
/// type, every value has exactly one coefficient, and equality, ordering and hashing compare
/// coefficients.
///
/// `Integral` and `Fraction` are [`typenum`] numbers such as `U8` or `U<8>`. `Repr` defaults to
/// the smallest signed integer that holds `Integral + Fraction` digits (see [`DigitCount`]). Any
/// other storage that holds them can be named explicitly: a wider one, or an unsigned one for
/// values that are never negative. An unsigned `Repr` holds as many digits as the signed one of
/// the same width, except that `u64` holds 19 digits and `i64` 18. The type has the size,
/// alignment and ABI of `Repr`.
///
/// ```
/// use arbitrary_numbers::{
///   Decimal,
///   typenum::{U8, U9, U10, U12},
/// };
///
/// type Price = Decimal<U10, U8>; // 18 digits, stored in an `i64`
/// type Quantity = Decimal<U12, U8>; // 20 digits, stored in an `i128`
/// type WidePrice = Decimal<U10, U8, i128>; // 18 digits in an explicitly chosen `i128`
/// type Volume = Decimal<U10, U9, u64>; // 19 digits, never negative, in a `u64`
///
/// assert_eq!(size_of::<Price>(), 8);
/// assert_eq!(size_of::<Quantity>(), 16);
/// assert_eq!(size_of::<WidePrice>(), 16);
/// assert_eq!(size_of::<Volume>(), 8);
/// ```
///
/// A combination that does not fit is rejected during type checking: zero digits, more than 38
/// digits, or a `Repr` too small for `Integral + Fraction` digits. With the default `Repr`, zero
/// digits or more than 38 digits fail wherever the type is written, because the default storage
/// must be computed. With an explicit `Repr`, a combination that does not fit can be written, but
/// its constants and constructors fail to compile, so no value of the type can be created.
///
/// ```compile_fail
/// use arbitrary_numbers::{
///   Decimal,
///   typenum::{U8, U10},
/// };
///
/// let _ = Decimal::<U10, U8, i32>::ZERO; // 18 digits do not fit in an `i32`
/// ```
///
/// Types with different `Repr` are distinct; [`widen`](Self::widen) and
/// [`try_convert`](Self::try_convert) convert between decimal types. The default `Repr` is meant
/// for concrete types, and generic code names `Repr`. `Decimal` itself has no bounds, so generic
/// code can store and pass `Decimal<I, F, R>` values without any. It needs `R: Storage` to copy,
/// compare or hash them, and adds `(I, F): Layout<R>` only where it uses items that depend on the
/// digit counts, such as formatting and conversions (see [`Layout`]).
///
/// # Operations
///
/// Every operation is exact: it returns the exact result or reports failure, and nothing rounds,
/// truncates or wraps. The operators `+`, `-`, `*` and `/` are not implemented; use
/// [`checked_add`](Self::checked_add), [`checked_sub`](Self::checked_sub) and
/// [`exact_mul`](Self::exact_mul). With a signed `Repr`, negation always succeeds because the
/// range is symmetric, so [`Neg`](core::ops::Neg) is implemented, along with
/// [`neg`](Self::neg), [`abs`](Self::abs) and [`is_negative`](Self::is_negative). An unsigned
/// `Repr` has none of them, and its `checked_sub` returns `None` below zero.
///
/// Conversions keep track of the sign. [`exact_mul`](Self::exact_mul) stores the product of two
/// unsigned storages in an unsigned storage and any other product in a signed one;
/// [`widen`](Self::widen) converts an unsigned storage to any storage but a signed storage only to
/// a signed one; and [`try_convert`](Self::try_convert) reports a negative value converted to an
/// unsigned storage as out of range.
///
/// Methods whose behavior depends only on the storage are `const fn`, generated for each storage,
/// so values can be computed at compile time:
///
/// ```
/// use arbitrary_numbers::{
///   Decimal,
///   typenum::{U8, U10},
/// };
///
/// type Price = Decimal<U10, U8>;
///
/// const TICK: Price = match Price::parse("0.01") {
///   Ok(tick) => tick,
///   Err(_) => panic!("invalid tick"),
/// };
/// const TWO_TICKS: Option<Price> = TICK.checked_add(TICK);
///
/// assert_eq!(TWO_TICKS, Price::from_repr(2_000_000));
/// assert_eq!(Price::MAX.checked_add(TICK), None);
/// ```
///
/// # Text
///
/// [`parse`](Self::parse) and [`FromStr`](core::str::FromStr) accept ASCII text with an optional
/// sign, point and exponent, and `_` separators as in Rust literals: the numeric strings of the
/// General Decimal Arithmetic specification and every form that [`f64`] parses, such as `-1.5`,
/// `+.5`, `5.`, `2.5E-3` and `1_000`. An unsigned `Repr` rejects a leading `-`. Infinity and NaN
/// are recognized and reported as [`NotFinite`](crate::ParseErrorKind::NotFinite), because a
/// decimal is always finite. The value must fit exactly: once the exponent has moved the point,
/// digits beyond `Integral` and `Fraction` are accepted only when they are leading or trailing
/// zeros; anything else is an error, never a rounded value. See [`parse`](Self::parse) for the
/// grammar.
///
/// [`Display`](fmt::Display) writes the shortest exact form: an optional `-`, the integral part
/// without leading zeros (at least one digit), then `.` and the fraction digits without trailing
/// zeros, omitted when they are all zero. It never uses exponent notation, and the text depends
/// only on the value, not on `Repr` or the build configuration. [`Debug`](fmt::Debug) writes the
/// same text. Text written for one type parses back into any type with at least as many integral
/// and fraction digits, except that a negative value needs a signed `Repr`.
///
/// The width, fill, alignment, `+` and `0` flags behave as they do for integers. The precision is
/// the minimum number of fraction digits: shorter fractions are padded with zeros, even beyond
/// `Fraction`.
///
/// <div class="warning">
///
/// Precision never rounds or truncates. Unlike floating-point formatting, `{:.1}` writes `1.25`
/// as `1.25`, not `1.2` or `1.3`.
///
/// </div>
///
/// ```
/// use arbitrary_numbers::{
///   Decimal,
///   typenum::{U4, U6},
/// };
///
/// type Amount = Decimal<U6, U4>;
///
/// let amount: Amount = "-001.2500".parse()?;
/// assert_eq!(amount, "-1_25e-2".parse()?);
/// assert_eq!(amount.to_string(), "-1.25");
/// assert_eq!(format!("{amount:.3}"), "-1.250");
/// assert_eq!(format!("{amount:.1}"), "-1.25");
/// assert_eq!(format!("{amount:>8}"), "   -1.25");
/// assert_eq!(format!("{amount:08.3}"), "-001.250");
/// assert_eq!(format!("{:+}", amount.abs()), "+1.25");
/// # Ok::<(), arbitrary_numbers::ParseError>(())
/// ```
#[repr(transparent)]
pub struct Decimal<Integral, Fraction, Repr = <Sum<Integral, Fraction> as DigitCount>::Signed> {
  repr: Repr,
  _digits: PhantomData<(Integral, Fraction)>,
}

impl<I, F, R> Decimal<I, F, R> {
  /// Wraps a coefficient without checking it. Callers ensure that the layout of the type holds
  /// and that `|repr| <= 10^(Integral + Fraction) - 1`.
  const fn from_coeff(repr: R) -> Self {
    Self {
      repr,
      _digits: PhantomData,
    }
  }
}

impl<I, F, R> Decimal<I, F, R>
where
  (I, F): Layout<R>,
{
  const INTEGRAL: u32 = <(I, F) as SealedLayout<R>>::INTEGRAL;
  const FRACTION: u32 = <(I, F) as SealedLayout<R>>::FRACTION;
  const DIGITS: u32 = <(I, F) as SealedLayout<R>>::DIGITS;
}

impl<I, F, R> Decimal<I, F, R>
where
  R: Storage,
{
  /// Multiplies two decimals exactly.
  ///
  /// The result has `Integral + I2` integral digits and `Fraction + F2` fraction digits, so the
  /// product always fits and is never rounded. It is stored in the smallest unsigned storage for
  /// its digits if both operands have unsigned storages, and in the smallest signed storage for
  /// its digits otherwise (see [`DigitCount`] and [`Pick`]). A product type with more than 38
  /// digits fails to compile.
  ///
  /// ```
  /// use arbitrary_numbers::{
  ///   Decimal,
  ///   typenum::{U8, U9, U10, U11, U12, U16, U17, U21, U22},
  /// };
  ///
  /// let price: Decimal<U10, U8> = "64250.5".parse()?;
  /// let quantity: Decimal<U12, U8> = "-0.0015".parse()?;
  /// let notional: Decimal<U22, U16> = price.exact_mul(quantity);
  /// assert_eq!(notional.to_string(), "-96.37575");
  ///
  /// // The product of two unsigned storages is unsigned.
  /// let price: Decimal<U10, U9, u64> = "64250.5".parse()?;
  /// let quantity: Decimal<U11, U8, u64> = "0.0015".parse()?;
  /// let notional: Decimal<U21, U17, u128> = price.exact_mul(quantity);
  /// assert_eq!(notional.to_string(), "96.37575");
  /// # Ok::<(), arbitrary_numbers::ParseError>(())
  /// ```
  #[must_use]
  #[expect(
    clippy::type_complexity,
    reason = "the product storage is computed from the operand storages and the digit count"
  )]
  pub fn exact_mul<I2, F2, R2>(
    self,
    rhs: Decimal<I2, F2, R2>,
  ) -> Decimal<
    Sum<I, I2>,
    Sum<F, F2>,
    <And<R::IsUnsigned, R2::IsUnsigned> as Pick<Sum<Sum<I, I2>, Sum<F, F2>>>>::Storage,
  >
  where
    R2: Storage,
    I: Add<I2>,
    F: Add<F2>,
    Sum<I, I2>: Add<Sum<F, F2>>,
    R::IsUnsigned: BitAnd<R2::IsUnsigned>,
    And<R::IsUnsigned, R2::IsUnsigned>: Pick<Sum<Sum<I, I2>, Sum<F, F2>>>,
  {
    // Every value has at most the digits of its type: the other constructors require the layout
    // of the type they create, and a product has at most as many digits as its operands
    // together, which is the digit count of its type. `DigitCount` caps that count at 38, so the
    // product is below 10^38 < 2^127 and fits the storage that `Pick` chooses for it. That
    // storage is unsigned only if both operands are, and then the product is not negative.
    let product = int::i128::mul(self.repr.to_i128(), rhs.repr.to_i128());
    Decimal::from_coeff(SealedStorage::from_i128(product))
  }
}

impl<I, F, R> Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Converts to a type with at least as many integral and fraction digits, and any storage that
  /// holds them, as long as a signed storage stays signed. Never fails.
  ///
  /// Converting to fewer integral or fraction digits fails to compile, and so does converting
  /// from a signed to an unsigned storage, which has no negative values; use
  /// [`try_convert`](Self::try_convert) instead. An unsigned storage converts to any storage.
  ///
  /// ```
  /// use arbitrary_numbers::{
  ///   Decimal,
  ///   typenum::{U2, U4, U8, U10},
  /// };
  ///
  /// let small: Decimal<U2, U4> = "-12.5".parse()?;
  /// let wide: Decimal<U10, U8, i128> = small.widen();
  /// assert_eq!(wide.repr(), -1_250_000_000);
  ///
  /// let unsigned: Decimal<U2, U4, u32> = "12.5".parse()?;
  /// let signed: Decimal<U10, U8> = unsigned.widen();
  /// assert_eq!(signed.repr(), 1_250_000_000);
  /// # Ok::<(), arbitrary_numbers::ParseError>(())
  /// ```
  #[must_use]
  pub fn widen<I2, F2, R2>(self) -> Decimal<I2, F2, R2>
  where
    R: WidenRepr<R2>,
    R2: Storage,
    (I2, F2): Layout<R2>,
    (I, F): Widen<(I2, F2)>,
  {
    // `I2 >= I` and `F2 >= F`, so the scaled coefficient has at most `I2 + F2` digits. `R2` is
    // unsigned only if `R` is, and then the coefficient is not negative.
    let scale = Decimal::<I2, F2, R2>::FRACTION - Self::FRACTION;
    let coeff = int::i128::mul_pow10(self.repr.to_i128(), scale);
    Decimal::from_coeff(R2::from_i128(coeff))
  }

  /// Converts to any decimal type, failing instead of losing digits.
  ///
  /// # Errors
  ///
  /// Returns [`ConvertError::OutOfRange`] if the value is outside the range of the target type,
  /// that is, if its integral part has more than `I2` digits, or if it is negative and `R2` is
  /// unsigned. Otherwise returns [`ConvertError::Inexact`] if the target has fewer fraction digits
  /// and a dropped digit is not zero. When both apply, the error is `OutOfRange`: a negative value
  /// converted to an unsigned storage is out of range even if dropping digits would leave zero,
  /// as for `-0.5` with no fraction digits.
  ///
  /// ```
  /// use arbitrary_numbers::{
  ///   ConvertError, Decimal,
  ///   typenum::{U0, U2, U3, U4, U10},
  /// };
  ///
  /// let value: Decimal<U10, U4> = "-12.5000".parse()?;
  /// let narrow: Decimal<U2, U2> = value.try_convert().unwrap();
  /// assert_eq!(narrow.to_string(), "-12.5");
  /// assert_eq!(
  ///   value.try_convert::<U2, U2, u16>(),
  ///   Err(ConvertError::OutOfRange)
  /// );
  ///
  /// let half: Decimal<U10, U4> = "-0.5".parse()?;
  /// assert_eq!(
  ///   half.try_convert::<U2, U0, u8>(),
  ///   Err(ConvertError::OutOfRange)
  /// );
  ///
  /// let too_large: Decimal<U10, U4> = "123".parse()?;
  /// assert_eq!(
  ///   too_large.try_convert::<U2, U3, i32>(),
  ///   Err(ConvertError::OutOfRange)
  /// );
  ///
  /// let precise: Decimal<U10, U4> = "1.2345".parse()?;
  /// assert_eq!(
  ///   precise.try_convert::<U2, U2, i16>(),
  ///   Err(ConvertError::Inexact)
  /// );
  /// # Ok::<(), arbitrary_numbers::ParseError>(())
  /// ```
  pub fn try_convert<I2, F2, R2>(self) -> Result<Decimal<I2, F2, R2>, ConvertError>
  where
    R2: Storage,
    (I2, F2): Layout<R2>,
  {
    let coeff = self.repr.to_i128();
    // Checked before scaling, so that a negative value that truncates to zero, such as `-0.5`,
    // is out of range rather than inexact. With this check, the range below is `0..=max` for an
    // unsigned target, and `R2::from_i128` never receives a negative value for one.
    if int::i128::is_negative(coeff) && !R2::SIGNED {
      return Err(ConvertError::OutOfRange);
    }
    let from = Self::FRACTION;
    let to = Decimal::<I2, F2, R2>::FRACTION;
    let max = int::i128::max_magnitude(Decimal::<I2, F2, R2>::DIGITS);
    let converted = if to >= from {
      match int::i128::checked_mul_pow10(coeff, to - from) {
        Some(scaled) if within(scaled, max) => scaled,
        _ => return Err(ConvertError::OutOfRange),
      }
    } else {
      // The truncated quotient is within `max` exactly when the value's integral part has at
      // most `I2` digits, so range is decided before exactness.
      let (quotient, remainder) = int::i128::div_rem_pow10(coeff, from - to);
      if !within(quotient, max) {
        return Err(ConvertError::OutOfRange);
      }
      if !int::i128::is_zero(remainder) {
        return Err(ConvertError::Inexact);
      }
      quotient
    };
    Ok(Decimal::from_coeff(R2::from_i128(converted)))
  }
}

/// Returns `true` if `|value| <= max`, for `max >= 0`.
const fn within(value: i128, max: i128) -> bool {
  int::i128::le(int::i128::neg(max), value) && int::i128::le(value, max)
}

impl<I, F, R> Clone for Decimal<I, F, R>
where
  R: Copy,
{
  #[inline]
  fn clone(&self) -> Self {
    *self
  }
}

impl<I, F, R> Copy for Decimal<I, F, R> where R: Copy {}

impl<I, F, R> PartialEq for Decimal<I, F, R>
where
  R: PartialEq,
{
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    self.repr == other.repr
  }
}

impl<I, F, R> Eq for Decimal<I, F, R> where R: Eq {}

impl<I, F, R> PartialOrd for Decimal<I, F, R>
where
  R: Ord,
{
  #[inline]
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<I, F, R> Ord for Decimal<I, F, R>
where
  R: Ord,
{
  #[inline]
  fn cmp(&self, other: &Self) -> Ordering {
    self.repr.cmp(&other.repr)
  }
}

impl<I, F, R> Hash for Decimal<I, F, R>
where
  R: Hash,
{
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.repr.hash(state);
  }
}

impl<I, F, R> fmt::Display for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Writes the shortest exact form, honoring width, fill, alignment, `+`, `0` and precision as
  /// described on [`Decimal`]. Precision never rounds or truncates.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut buf = [0; int::DIGIT_BUF_LEN];
    let start = self.repr.write_digits(&mut buf);
    format::write(
      f,
      self.repr.is_negative(),
      &buf[start..],
      Self::FRACTION as usize,
    )
  }
}

impl<I, F, R> fmt::Debug for Decimal<I, F, R>
where
  R: Storage,
  (I, F): Layout<R>,
{
  /// Writes the same text as [`Display`](fmt::Display).
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self, f)
  }
}
