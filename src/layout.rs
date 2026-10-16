//! Type-level digit counts and the storage types that hold them.

use core::{hash::Hash, ops::Add};

use typenum::{B0, B1, IsLessOrEqual, LeEq, Sum, Unsigned};

use sealed::{
  FractionNotReduced, IntegralNotReduced, SealedDigitCount, SealedFitsDigits, SealedPick,
  SealedWiden, SealedWidenRepr,
};
pub(crate) use sealed::{SealedLayout, SealedStorage};

use crate::int::{self, DigitBuf};

mod sealed {
  use crate::int::DigitBuf;

  /// Seals [`Storage`](super::Storage) and gives storage-generic code access to the integer core.
  // clippy's `wrong_self_convention` requires `Copy` for `is_*` methods that take `self`.
  pub trait SealedStorage: Copy {
    /// `B1` for an unsigned storage, `B0` for a signed one.
    type IsUnsigned: typenum::Bit;

    /// `true` for a signed storage.
    const SIGNED: bool = !<Self::IsUnsigned as typenum::Bit>::BOOL;

    /// Returns `true` if the value is negative, which an unsigned value never is.
    fn is_negative(self) -> bool;

    /// Widens the value to `i128`.
    fn to_i128(self) -> i128;

    /// Narrows `value`, which must be representable: never negative for an unsigned storage.
    fn from_i128(value: i128) -> Self;

    /// Writes the decimal digits of the magnitude to the end of `buf` and returns the index of
    /// the first digit.
    fn write_digits(self, buf: &mut DigitBuf) -> usize;
  }

  /// Seals [`DigitCount`](super::DigitCount).
  pub trait SealedDigitCount {}

  /// Seals [`FitsDigits`](super::FitsDigits).
  pub trait SealedFitsDigits<Repr> {}

  /// Seals [`Layout`](super::Layout) and carries the digit counts as constants.
  pub trait SealedLayout<Repr> {
    /// The number of integral digits.
    const INTEGRAL: u32;
    /// The number of fraction digits.
    const FRACTION: u32;
    /// The total number of digits, `INTEGRAL + FRACTION`.
    const DIGITS: u32;
  }

  /// Seals [`Pick`](super::Pick).
  pub trait SealedPick<Digits> {}

  /// Seals [`Widen`](super::Widen).
  pub trait SealedWiden<Target> {}

  /// Seals [`WidenRepr`](super::WidenRepr).
  pub trait SealedWidenRepr<Target> {}

  /// Implemented only for `B1`, the result of `Integral <= I2` when widening is possible.
  #[diagnostic::on_unimplemented(
    message = "`widen` cannot reduce the number of integral digits",
    label = "the target `Integral` is smaller than the source `Integral`",
    note = "use `try_convert` for a conversion that can fail"
  )]
  pub trait IntegralNotReduced {}

  impl IntegralNotReduced for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Fraction <= F2` when widening is possible.
  #[diagnostic::on_unimplemented(
    message = "`widen` cannot reduce the number of fraction digits",
    label = "the target `Fraction` is smaller than the source `Fraction`",
    note = "use `try_convert` for a conversion that can fail"
  )]
  pub trait FractionNotReduced {}

  impl FractionNotReduced for typenum::B1 {}
}

/// A native integer type that can store the coefficient of a [`Decimal`](crate::Decimal).
///
/// Implemented for the signed `i8`, `i16`, `i32`, `i64` and `i128`, and the unsigned `u8`, `u16`,
/// `u32`, `u64` and `u128`. A decimal with an unsigned storage is never negative. This trait is
/// sealed and cannot be implemented outside this crate.
pub trait Storage: SealedStorage + Copy + Ord + Hash {}

/// Maps a type-level number of decimal digits to the smallest signed and the smallest unsigned
/// integer types that hold every value with that many digits.
///
/// | Digits | `Signed` | `Unsigned` |
/// |---|---|---|
/// | 1-2 | `i8` | `u8` |
/// | 3-4 | `i16` | `u16` |
/// | 5-9 | `i32` | `u32` |
/// | 10-18 | `i64` | `u64` |
/// | 19 | `i128` | `u64` |
/// | 20-38 | `i128` | `u128` |
///
/// Implemented for `U1` through `U38`. [`Decimal`](crate::Decimal) uses `Signed` as its default
/// storage, and [`exact_mul`](crate::Decimal::exact_mul) stores a product of two unsigned storages
/// in `Unsigned`. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "a `Decimal` must have between 1 and 38 digits in total",
  label = "`Integral + Fraction` is zero or greater than 38"
)]
pub trait DigitCount: SealedDigitCount {
  /// The smallest signed integer type that holds every value with `Self` decimal digits.
  type Signed: Storage;

  /// The smallest unsigned integer type that holds every value with `Self` decimal digits.
  type Unsigned: Storage;
}

/// Marks that `Repr` holds every value with `Self` decimal digits.
///
/// `Self` is a type-level number of digits. Implemented for every pair that fits: `i8` and `u8`
/// hold up to 2 digits, `i16` and `u16` up to 4, `i32` and `u32` up to 9, `i64` up to 18, `u64` up
/// to 19, and `i128` and `u128` up to 38. This trait is sealed and cannot be implemented outside
/// this crate.
#[diagnostic::on_unimplemented(
  message = "the storage `{Repr}` cannot hold `Integral + Fraction` decimal digits",
  label = "`Integral + Fraction` does not fit in `{Repr}`",
  note = "`Integral + Fraction` must be at least 1; `i8` and `u8` hold up to 2 digits, `i16` and `u16` up to 4, `i32` and `u32` up to 9, `i64` up to 18, `u64` up to 19, and `i128` and `u128` up to 38"
)]
pub trait FitsDigits<Repr>: SealedFitsDigits<Repr> {}

/// Requires that `Repr` holds `Integral + Fraction` decimal digits, implemented for the pair
/// `(Integral, Fraction)` of type-level numbers.
///
/// [`Decimal`](crate::Decimal) requires `(Integral, Fraction): Layout<Repr>` on its constants,
/// constructors, `const fn` methods, formatting and conversions, not on the type itself. Generic
/// code needs no bounds to store and pass `Decimal<I, F, R>` values and only `R: Storage` to copy,
/// compare and hash them; it adds `(I, F): Layout<R>` where it uses the other items:
///
/// ```
/// use arbitrary_numbers::{
///   Decimal, Layout, Storage,
///   typenum::{U2, U4},
/// };
///
/// // Storing a value needs no bounds.
/// struct Entry<I, F, R> {
///   label: &'static str,
///   amount: Decimal<I, F, R>,
/// }
///
/// // Formatting depends on the number of fraction digits, so it needs the layout.
/// fn describe<I, F, R>(entry: &Entry<I, F, R>) -> String
/// where
///   R: Storage,
///   (I, F): Layout<R>,
/// {
///   format!("{}: {}", entry.label, entry.amount)
/// }
///
/// let entry = Entry {
///   label: "fee",
///   amount: "12.5".parse::<Decimal<U2, U4>>().unwrap(),
/// };
/// assert_eq!(describe(&entry), "fee: 12.5");
/// ```
///
/// For `typenum` unsigned numbers `Integral` and `Fraction`, the bound holds exactly when
/// `Integral + Fraction` implements [`FitsDigits<Repr>`]. When it does not, the compiler reports
/// the diagnostic of [`FitsDigits`], or, for a constant or method whose impl requires the bound,
/// that the impl's bounds are not satisfied. This trait is sealed and cannot be implemented
/// outside this crate.
pub trait Layout<Repr>: SealedLayout<Repr> {}

impl<I, F, R> SealedLayout<R> for (I, F)
where
  I: Unsigned + Add<F>,
  F: Unsigned,
  Sum<I, F>: FitsDigits<R>,
{
  const INTEGRAL: u32 = I::U32;
  const FRACTION: u32 = F::U32;
  const DIGITS: u32 = I::U32 + F::U32;
}

impl<I, F, R> Layout<R> for (I, F)
where
  I: Unsigned + Add<F>,
  F: Unsigned,
  Sum<I, F>: FitsDigits<R>,
{
}

/// Picks the smallest storage of one sign for a type-level number of decimal digits.
///
/// Implemented for the `typenum` bits: `B0` picks [`DigitCount::Signed`] and `B1` picks
/// [`DigitCount::Unsigned`]. [`Decimal::exact_mul`](crate::Decimal::exact_mul) picks with `B1`
/// when both operands have unsigned storages and with `B0` otherwise. This trait is sealed and
/// cannot be implemented outside this crate.
pub trait Pick<Digits>: SealedPick<Digits> {
  /// The smallest storage of the picked sign that holds every value with `Digits` decimal digits.
  type Storage: Storage;
}

impl<D> SealedPick<D> for B0 {}

impl<D: DigitCount> Pick<D> for B0 {
  type Storage = D::Signed;
}

impl<D> SealedPick<D> for B1 {}

impl<D: DigitCount> Pick<D> for B1 {
  type Storage = D::Unsigned;
}

/// Requires that a decimal with the digit counts `Self = (Integral, Fraction)` converts to one with
/// the digit counts `Target = (I2, F2)` without losing digits: `Integral <= I2` and
/// `Fraction <= F2`.
///
/// Used by [`Decimal::widen`](crate::Decimal::widen). This trait is sealed and cannot be
/// implemented outside this crate.
pub trait Widen<Target>: SealedWiden<Target> {}

impl<I, F, I2, F2> SealedWiden<(I2, F2)> for (I, F)
where
  I: IsLessOrEqual<I2>,
  F: IsLessOrEqual<F2>,
  LeEq<I, I2>: IntegralNotReduced,
  LeEq<F, F2>: FractionNotReduced,
{
}

impl<I, F, I2, F2> Widen<(I2, F2)> for (I, F)
where
  I: IsLessOrEqual<I2>,
  F: IsLessOrEqual<F2>,
  LeEq<I, I2>: IntegralNotReduced,
  LeEq<F, F2>: FractionNotReduced,
{
}

/// Requires that a value stored in `Self` converts to the storage `Target` whatever its sign: an
/// unsigned storage converts to any storage, and a signed storage only to a signed one.
///
/// Used by [`Decimal::widen`](crate::Decimal::widen), which never fails. A negative value has no
/// unsigned representation, so a conversion from a signed to an unsigned storage goes through
/// [`Decimal::try_convert`](crate::Decimal::try_convert). This trait covers only the sign;
/// [`Layout`] checks that `Target` holds the digits. This trait is sealed and cannot be implemented
/// outside this crate.
#[diagnostic::on_unimplemented(
  message = "`widen` cannot convert from the signed storage `{Self}` to the unsigned storage `{Target}`",
  note = "use `try_convert` for a conversion that can fail"
)]
pub trait WidenRepr<Target>: SealedWidenRepr<Target> {}

macro_rules! impl_storage {
  (signed: $($int:ident),+; unsigned: $($uint:ident),+ $(,)?) => {
    $(
      impl SealedStorage for $int {
        type IsUnsigned = B0;

        #[inline]
        fn is_negative(self) -> bool {
          int::$int::is_negative(self)
        }

        #[inline]
        fn to_i128(self) -> i128 {
          int::$int::to_i128(self)
        }

        #[inline]
        fn from_i128(value: i128) -> Self {
          int::$int::from_i128(value)
        }

        #[inline]
        fn write_digits(self, buf: &mut DigitBuf) -> usize {
          int::$int::write_digits(self, buf)
        }
      }

      impl Storage for $int {}
    )+
    $(
      impl SealedStorage for $uint {
        type IsUnsigned = B1;

        #[inline]
        fn is_negative(self) -> bool {
          false
        }

        #[inline]
        fn to_i128(self) -> i128 {
          int::$uint::to_i128(self)
        }

        #[inline]
        fn from_i128(value: i128) -> Self {
          int::$uint::from_i128(value)
        }

        #[inline]
        fn write_digits(self, buf: &mut DigitBuf) -> usize {
          int::$uint::write_digits(self, buf)
        }
      }

      impl Storage for $uint {}
    )+
  };
}

impl_storage!(signed: i8, i16, i32, i64, i128; unsigned: u8, u16, u32, u64, u128);

/// Implements [`WidenRepr`] from every unsigned storage to every storage, and from every signed
/// storage to every signed storage.
macro_rules! impl_widen_repr {
  (signed: [$($int:ident),+], unsigned: [$($uint:ident),+] $(,)?) => {
    impl_widen_repr!(@from [$($uint),+] to [$($int,)+ $($uint),+]);
    impl_widen_repr!(@from [$($int),+] to [$($int),+]);
  };
  (@from [$($source:ident),+] to $targets:tt) => {
    $(impl_widen_repr!(@pairs $source to $targets);)+
  };
  (@pairs $source:ident to [$($target:ident),+]) => {
    $(
      impl SealedWidenRepr<$target> for $source {}

      impl WidenRepr<$target> for $source {}
    )+
  };
}

impl_widen_repr!(
  signed: [i8, i16, i32, i64, i128],
  unsigned: [u8, u16, u32, u64, u128],
);

/// Passes the digit-count table to `$callback`.
///
/// Each row is `Digits = value => [Signed, Wider...], [Unsigned, Wider...]`: the `typenum`
/// constant for a number of digits, its value, and, for each sign, every storage that holds that
/// many digits, smallest first. The first storage of each list is the `DigitCount` mapping; every
/// listed storage gets `FitsDigits`.
macro_rules! digit_table {
  ($callback:ident) => {
    $callback! {
      U1 = 1 => [i8, i16, i32, i64, i128], [u8, u16, u32, u64, u128],
      U2 = 2 => [i8, i16, i32, i64, i128], [u8, u16, u32, u64, u128],
      U3 = 3 => [i16, i32, i64, i128], [u16, u32, u64, u128],
      U4 = 4 => [i16, i32, i64, i128], [u16, u32, u64, u128],
      U5 = 5 => [i32, i64, i128], [u32, u64, u128],
      U6 = 6 => [i32, i64, i128], [u32, u64, u128],
      U7 = 7 => [i32, i64, i128], [u32, u64, u128],
      U8 = 8 => [i32, i64, i128], [u32, u64, u128],
      U9 = 9 => [i32, i64, i128], [u32, u64, u128],
      U10 = 10 => [i64, i128], [u64, u128],
      U11 = 11 => [i64, i128], [u64, u128],
      U12 = 12 => [i64, i128], [u64, u128],
      U13 = 13 => [i64, i128], [u64, u128],
      U14 = 14 => [i64, i128], [u64, u128],
      U15 = 15 => [i64, i128], [u64, u128],
      U16 = 16 => [i64, i128], [u64, u128],
      U17 = 17 => [i64, i128], [u64, u128],
      U18 = 18 => [i64, i128], [u64, u128],
      U19 = 19 => [i128], [u64, u128],
      U20 = 20 => [i128], [u128],
      U21 = 21 => [i128], [u128],
      U22 = 22 => [i128], [u128],
      U23 = 23 => [i128], [u128],
      U24 = 24 => [i128], [u128],
      U25 = 25 => [i128], [u128],
      U26 = 26 => [i128], [u128],
      U27 = 27 => [i128], [u128],
      U28 = 28 => [i128], [u128],
      U29 = 29 => [i128], [u128],
      U30 = 30 => [i128], [u128],
      U31 = 31 => [i128], [u128],
      U32 = 32 => [i128], [u128],
      U33 = 33 => [i128], [u128],
      U34 = 34 => [i128], [u128],
      U35 = 35 => [i128], [u128],
      U36 = 36 => [i128], [u128],
      U37 = 37 => [i128], [u128],
      U38 = 38 => [i128], [u128],
    }
  };
}

macro_rules! impl_digits {
  ($(
    $digits:ident = $value:literal =>
      [$signed:ident $(, $wider_signed:ident)*],
      [$unsigned:ident $(, $wider_unsigned:ident)*]
  ),+ $(,)?) => {$(
    impl SealedDigitCount for typenum::consts::$digits {}

    impl DigitCount for typenum::consts::$digits {
      type Signed = $signed;
      type Unsigned = $unsigned;
    }

    impl_digits!(@fits $digits: $signed $(, $wider_signed)*, $unsigned $(, $wider_unsigned)*);
  )+};
  (@fits $digits:ident: $($repr:ident),+) => {$(
    impl SealedFitsDigits<$repr> for typenum::consts::$digits {}

    impl FitsDigits<$repr> for typenum::consts::$digits {}
  )+};
}

digit_table!(impl_digits);

#[cfg(test)]
mod tests;
