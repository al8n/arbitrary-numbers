//! Type-level digit counts and the storage types that hold them.

use core::{hash::Hash, ops::Add};

use typenum::{B0, B1, IsLessOrEqual, LeEq, Sum, Unsigned};

use sealed::{
  ConvertFrom, FractionNotReduced, IntegralNotReduced, SealedDigitCount, SealedFitsDigits,
  SealedPick, SealedWiden, SealedWidenRepr,
};
pub(crate) use sealed::{SealedLayout, SealedStorage};

use crate::{
  i192, i256, i320,
  int::{self, DigitBuf},
  u192, u256, u320,
};

mod sealed {
  use crate::{i192, i256, i320, int::DigitBuf, u192, u256, u320};

  /// Seals [`Storage`](super::Storage) and gives storage-generic code access to the integer core.
  ///
  /// Every storage converts to every storage: `convert` and `checked_convert`, implemented for
  /// each source storage, call the `ConvertFrom` implementation of the target, which this trait
  /// requires for every source. Storage-generic code is not `const`, because trait methods cannot
  /// be.
  // clippy's `wrong_self_convention` requires `Copy` for `is_*` methods that take `self`.
  pub trait SealedStorage:
    Copy
    + ConvertFrom<i8>
    + ConvertFrom<i16>
    + ConvertFrom<i32>
    + ConvertFrom<i64>
    + ConvertFrom<i128>
    + ConvertFrom<i192>
    + ConvertFrom<i256>
    + ConvertFrom<i320>
    + ConvertFrom<u8>
    + ConvertFrom<u16>
    + ConvertFrom<u32>
    + ConvertFrom<u64>
    + ConvertFrom<u128>
    + ConvertFrom<u192>
    + ConvertFrom<u256>
    + ConvertFrom<u320>
  {
    /// `B1` for an unsigned storage, `B0` for a signed one.
    type IsUnsigned: typenum::Bit;

    /// `true` for a signed storage.
    const SIGNED: bool = !<Self::IsUnsigned as typenum::Bit>::BOOL;

    /// Returns `true` if the value is negative, which an unsigned value never is.
    fn is_negative(self) -> bool;

    /// Returns `true` if the value is zero.
    fn is_zero(self) -> bool;

    /// Returns `true` if `|self| <= 10^digits - 1`. Requires `10^digits` to be representable.
    fn within_digits(self, digits: u32) -> bool;

    /// `self * rhs`. Requires the product to be representable.
    fn mul(self, rhs: Self) -> Self;

    /// `self * 10^exp`. Requires the product to be representable.
    fn mul_pow10(self, exp: u32) -> Self;

    /// `self * 10^exp`, or `None` if the product is not representable. Requires `10^exp` to be
    /// representable.
    fn checked_mul_pow10(self, exp: u32) -> Option<Self>;

    /// Quotient and remainder of `self / 10^exp`, rounding toward zero; both have the sign of
    /// `self`. Requires `10^exp` to be representable.
    fn div_rem_pow10(self, exp: u32) -> (Self, Self);

    /// Writes the decimal digits of the magnitude to the end of `buf` and returns the index of
    /// the first digit.
    fn write_digits(self, buf: &mut DigitBuf) -> usize;

    /// Converts the value to `Target`. Requires it to be representable there, so not negative for
    /// an unsigned `Target`.
    fn convert<Target: SealedStorage>(self) -> Target;

    /// Converts the value to `Target`, or returns `None` if it is not representable there.
    fn checked_convert<Target: SealedStorage>(self) -> Option<Target>;
  }

  /// Converts values of the storage `Source` to this storage. Implemented for every ordered pair
  /// of storages.
  pub trait ConvertFrom<Source>: Sized {
    /// Converts `value`. Requires it to be representable in this storage, so not negative for an
    /// unsigned one.
    fn convert_from(value: Source) -> Self;

    /// Converts `value`, or returns `None` if it is not representable in this storage.
    fn checked_convert_from(value: Source) -> Option<Self>;
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

/// An integer type that can store the coefficient of a [`Decimal`](crate::Decimal).
///
/// Implemented for the native signed integers `i8`, `i16`, `i32`, `i64` and `i128`, the native
/// unsigned integers `u8`, `u16`, `u32`, `u64` and `u128`, and the wide integers
/// [`i192`](crate::i192), [`i256`](crate::i256), [`i320`](crate::i320), [`u192`](crate::u192),
/// [`u256`](crate::u256) and [`u320`](crate::u320). A decimal with an unsigned storage is never
/// negative. This trait is sealed and cannot be implemented outside this crate.
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
/// | 39-57 | [`i192`](crate::i192) | [`u192`](crate::u192) |
/// | 58-76 | [`i256`](crate::i256) | [`u256`](crate::u256) |
/// | 77 | [`i320`](crate::i320) | [`u256`](crate::u256) |
/// | 78-96 | [`i320`](crate::i320) | [`u320`](crate::u320) |
///
/// Implemented for `U1` through `U96`. [`Decimal`](crate::Decimal) uses `Signed` as its default
/// storage, and [`exact_mul`](crate::Decimal::exact_mul) stores a product of two unsigned storages
/// in `Unsigned`. This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "a `Decimal` must have between 1 and 96 digits in total",
  label = "`Integral + Fraction` is zero or greater than 96"
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
/// to 19, `i128` and `u128` up to 38, `i192` and `u192` up to 57, `i256` up to 76, `u256` up to
/// 77, and `i320` and `u320` up to 96. This trait is sealed and cannot be implemented outside this
/// crate.
// The diagnostic does not print `{Repr}`, which spells the wide storages out as `Uint` or `Int`
// with `typenum` bits and a limb array.
#[diagnostic::on_unimplemented(
  message = "the storage `Repr` cannot hold `Integral + Fraction` decimal digits",
  label = "`Integral + Fraction` does not fit in `Repr`",
  note = "`Integral + Fraction` must be at least 1; `i8` and `u8` hold up to 2 digits, `i16` and `u16` up to 4, `i32` and `u32` up to 9, `i64` up to 18, `u64` up to 19, `i128` and `u128` up to 38, `i192` and `u192` up to 57, `i256` up to 76, `u256` up to 77, and `i320` and `u320` up to 96",
  note = "the compiler writes `u192`, `u256` and `u320` as `Uint` with the limbs `[u64; 3]`, `[u64; 4]` and `[u64; 5]`, and `i192`, `i256` and `i320` as `Int` with the same limbs"
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
// As for `FitsDigits`, the diagnostic does not print the storages.
#[diagnostic::on_unimplemented(
  message = "`widen` cannot convert from a signed storage to an unsigned storage",
  note = "use `try_convert` for a conversion that can fail",
  note = "the compiler writes `u192`, `u256` and `u320` as `Uint` with the limbs `[u64; 3]`, `[u64; 4]` and `[u64; 5]`, and `i192`, `i256` and `i320` as `Int` with the same limbs"
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
        fn within_digits(self, digits: u32) -> bool {
          let max = int::$int::max_magnitude(digits);
          int::$int::le(int::$int::neg(max), self) && int::$int::le(self, max)
        }

        impl_storage!(@shared $int);
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
        fn within_digits(self, digits: u32) -> bool {
          int::$uint::le(self, int::$uint::max_magnitude(digits))
        }

        impl_storage!(@shared $uint);
      }

      impl Storage for $uint {}
    )+
  };
  (@shared $repr:ident) => {
    #[inline]
    fn is_zero(self) -> bool {
      int::$repr::is_zero(self)
    }

    #[inline]
    fn mul(self, rhs: Self) -> Self {
      int::$repr::mul(self, rhs)
    }

    #[inline]
    fn mul_pow10(self, exp: u32) -> Self {
      int::$repr::mul_pow10(self, exp)
    }

    #[inline]
    fn checked_mul_pow10(self, exp: u32) -> Option<Self> {
      int::$repr::checked_mul_pow10(self, exp)
    }

    #[inline]
    fn div_rem_pow10(self, exp: u32) -> (Self, Self) {
      int::$repr::div_rem_pow10(self, exp)
    }

    #[inline]
    fn write_digits(self, buf: &mut DigitBuf) -> usize {
      int::$repr::write_digits(self, buf)
    }

    #[inline]
    fn convert<Target: SealedStorage>(self) -> Target {
      <Target as ConvertFrom<$repr>>::convert_from(self)
    }

    #[inline]
    fn checked_convert<Target: SealedStorage>(self) -> Option<Target> {
      <Target as ConvertFrom<$repr>>::checked_convert_from(self)
    }
  };
}

impl_storage!(
  signed: i8, i16, i32, i64, i128, i192, i256, i320;
  unsigned: u8, u16, u32, u64, u128, u192, u256, u320,
);

/// Implements [`ConvertFrom`] for every ordered pair of the listed storages, each written as
/// `(name sign kind)` with the sign `signed` or `unsigned` and the kind `native` or `wide`.
///
/// Between native storages, a conversion is a cast or a native `TryFrom`. Any other conversion
/// goes through limbs: the source's limbs are extended by the source's sign, or truncated, to the
/// target's limb count. The checked conversion fails when that changes the value or, between signs,
/// when the value is negative or reaches the sign bit of a signed target; the target then checks
/// its own range, which a native storage narrower than two limbs needs.
macro_rules! impl_convert {
  ([$($storage:tt),+ $(,)?]) => {
    impl_convert!(@sources [$($storage),+] to [$($storage),+]);
  };
  (@sources [$($source:tt),+] to $targets:tt) => {
    $(impl_convert!(@pairs $source to $targets);)+
  };
  (@pairs $source:tt to [$($target:tt),+]) => {
    $(impl_convert!(@pair $source to $target);)+
  };
  (@pair
    ($source:ident $source_sign:ident native) to ($target:ident $target_sign:ident native)
  ) => {
    impl ConvertFrom<$source> for $target {
      #[inline]
      fn convert_from(value: $source) -> Self {
        value as $target
      }

      #[inline]
      fn checked_convert_from(value: $source) -> Option<Self> {
        <$target>::try_from(value).ok()
      }
    }
  };
  (@pair
    ($source:ident $source_sign:ident $source_kind:ident)
    to ($target:ident $target_sign:ident $target_kind:ident)
  ) => {
    impl ConvertFrom<$source> for $target {
      #[inline]
      fn convert_from(value: $source) -> Self {
        int::$target::from_limbs(impl_convert!(@resize $source_sign int::$source::to_limbs(value)))
      }

      #[inline]
      fn checked_convert_from(value: $source) -> Option<Self> {
        let limbs = int::$source::to_limbs(value);
        match impl_convert!(@checked_resize $source_sign $target_sign limbs) {
          Some(limbs) => int::$target::checked_from_limbs(limbs),
          None => None,
        }
      }
    }
  };
  (@resize unsigned $limbs:expr) => {
    int::limbs::resize($limbs)
  };
  (@resize signed $limbs:expr) => {
    int::signed_limbs::resize($limbs)
  };
  (@checked_resize unsigned unsigned $limbs:expr) => {
    int::limbs::checked_resize($limbs)
  };
  (@checked_resize signed signed $limbs:expr) => {
    int::signed_limbs::checked_resize($limbs)
  };
  (@checked_resize signed unsigned $limbs:expr) => {
    int::signed_limbs::checked_to_unsigned($limbs)
  };
  (@checked_resize unsigned signed $limbs:expr) => {
    int::signed_limbs::checked_from_unsigned($limbs)
  };
}

impl_convert!([
  (i8 signed native),
  (i16 signed native),
  (i32 signed native),
  (i64 signed native),
  (i128 signed native),
  (u8 unsigned native),
  (u16 unsigned native),
  (u32 unsigned native),
  (u64 unsigned native),
  (u128 unsigned native),
  (i192 signed wide),
  (i256 signed wide),
  (i320 signed wide),
  (u192 unsigned wide),
  (u256 unsigned wide),
  (u320 unsigned wide),
]);

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
  signed: [i8, i16, i32, i64, i128, i192, i256, i320],
  unsigned: [u8, u16, u32, u64, u128, u192, u256, u320],
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
      U1 = 1 =>
        [i8, i16, i32, i64, i128, i192, i256, i320],
        [u8, u16, u32, u64, u128, u192, u256, u320],
      U2 = 2 =>
        [i8, i16, i32, i64, i128, i192, i256, i320],
        [u8, u16, u32, u64, u128, u192, u256, u320],
      U3 = 3 => [i16, i32, i64, i128, i192, i256, i320], [u16, u32, u64, u128, u192, u256, u320],
      U4 = 4 => [i16, i32, i64, i128, i192, i256, i320], [u16, u32, u64, u128, u192, u256, u320],
      U5 = 5 => [i32, i64, i128, i192, i256, i320], [u32, u64, u128, u192, u256, u320],
      U6 = 6 => [i32, i64, i128, i192, i256, i320], [u32, u64, u128, u192, u256, u320],
      U7 = 7 => [i32, i64, i128, i192, i256, i320], [u32, u64, u128, u192, u256, u320],
      U8 = 8 => [i32, i64, i128, i192, i256, i320], [u32, u64, u128, u192, u256, u320],
      U9 = 9 => [i32, i64, i128, i192, i256, i320], [u32, u64, u128, u192, u256, u320],
      U10 = 10 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U11 = 11 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U12 = 12 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U13 = 13 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U14 = 14 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U15 = 15 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U16 = 16 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U17 = 17 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U18 = 18 => [i64, i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U19 = 19 => [i128, i192, i256, i320], [u64, u128, u192, u256, u320],
      U20 = 20 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U21 = 21 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U22 = 22 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U23 = 23 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U24 = 24 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U25 = 25 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U26 = 26 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U27 = 27 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U28 = 28 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U29 = 29 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U30 = 30 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U31 = 31 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U32 = 32 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U33 = 33 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U34 = 34 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U35 = 35 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U36 = 36 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U37 = 37 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U38 = 38 => [i128, i192, i256, i320], [u128, u192, u256, u320],
      U39 = 39 => [i192, i256, i320], [u192, u256, u320],
      U40 = 40 => [i192, i256, i320], [u192, u256, u320],
      U41 = 41 => [i192, i256, i320], [u192, u256, u320],
      U42 = 42 => [i192, i256, i320], [u192, u256, u320],
      U43 = 43 => [i192, i256, i320], [u192, u256, u320],
      U44 = 44 => [i192, i256, i320], [u192, u256, u320],
      U45 = 45 => [i192, i256, i320], [u192, u256, u320],
      U46 = 46 => [i192, i256, i320], [u192, u256, u320],
      U47 = 47 => [i192, i256, i320], [u192, u256, u320],
      U48 = 48 => [i192, i256, i320], [u192, u256, u320],
      U49 = 49 => [i192, i256, i320], [u192, u256, u320],
      U50 = 50 => [i192, i256, i320], [u192, u256, u320],
      U51 = 51 => [i192, i256, i320], [u192, u256, u320],
      U52 = 52 => [i192, i256, i320], [u192, u256, u320],
      U53 = 53 => [i192, i256, i320], [u192, u256, u320],
      U54 = 54 => [i192, i256, i320], [u192, u256, u320],
      U55 = 55 => [i192, i256, i320], [u192, u256, u320],
      U56 = 56 => [i192, i256, i320], [u192, u256, u320],
      U57 = 57 => [i192, i256, i320], [u192, u256, u320],
      U58 = 58 => [i256, i320], [u256, u320],
      U59 = 59 => [i256, i320], [u256, u320],
      U60 = 60 => [i256, i320], [u256, u320],
      U61 = 61 => [i256, i320], [u256, u320],
      U62 = 62 => [i256, i320], [u256, u320],
      U63 = 63 => [i256, i320], [u256, u320],
      U64 = 64 => [i256, i320], [u256, u320],
      U65 = 65 => [i256, i320], [u256, u320],
      U66 = 66 => [i256, i320], [u256, u320],
      U67 = 67 => [i256, i320], [u256, u320],
      U68 = 68 => [i256, i320], [u256, u320],
      U69 = 69 => [i256, i320], [u256, u320],
      U70 = 70 => [i256, i320], [u256, u320],
      U71 = 71 => [i256, i320], [u256, u320],
      U72 = 72 => [i256, i320], [u256, u320],
      U73 = 73 => [i256, i320], [u256, u320],
      U74 = 74 => [i256, i320], [u256, u320],
      U75 = 75 => [i256, i320], [u256, u320],
      U76 = 76 => [i256, i320], [u256, u320],
      U77 = 77 => [i320], [u256, u320],
      U78 = 78 => [i320], [u320],
      U79 = 79 => [i320], [u320],
      U80 = 80 => [i320], [u320],
      U81 = 81 => [i320], [u320],
      U82 = 82 => [i320], [u320],
      U83 = 83 => [i320], [u320],
      U84 = 84 => [i320], [u320],
      U85 = 85 => [i320], [u320],
      U86 = 86 => [i320], [u320],
      U87 = 87 => [i320], [u320],
      U88 = 88 => [i320], [u320],
      U89 = 89 => [i320], [u320],
      U90 = 90 => [i320], [u320],
      U91 = 91 => [i320], [u320],
      U92 = 92 => [i320], [u320],
      U93 = 93 => [i320], [u320],
      U94 = 94 => [i320], [u320],
      U95 = 95 => [i320], [u320],
      U96 = 96 => [i320], [u320],
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
