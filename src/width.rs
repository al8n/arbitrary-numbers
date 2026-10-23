//! The width relation between a type-level number of bits and an integer storage, and the storages
//! of [`Uint`](crate::Uint) and [`Int`](crate::Int).

use core::{cmp::Ordering, fmt, hash::Hasher, ops::Shl};

use typenum::{Const, IsEqual, IsLessOrEqual, LeEq, NonZero, Shleft, ToUInt, U, U6, Unsigned};

use sealed::{BitsFit, ReprFilled, SealedFillsRepr, SealedFitsBits, SealedIntRepr, SealedUintRepr};

use crate::{
  int::{limbs, signed_limbs},
  layout::Token,
};

mod sealed {
  use core::{cmp::Ordering, fmt, hash::Hasher};

  use crate::layout::Token;

  /// Seals [`FitsBits`](super::FitsBits).
  pub trait SealedFitsBits<Repr> {}

  /// Seals [`FillsRepr`](super::FillsRepr).
  pub trait SealedFillsRepr<Repr> {}

  /// Implemented only for `B1`, the result of `Bits <= 64 * N` when `[u64; N]` holds `Bits` bits.
  // The message and label name no storage, so that every storage can share them.
  #[diagnostic::on_unimplemented(
    message = "the storage cannot hold `Bits` bits",
    label = "`Bits` is greater than the bits of the storage",
    note = "`[u64; N]` holds `64 * N` bits"
  )]
  pub trait BitsFit {}

  impl BitsFit for typenum::B1 {}

  /// Implemented only for `B1`, the result of `Bits == 64 * N` when `Bits` fills `[u64; N]`.
  #[diagnostic::on_unimplemented(
    message = "`Bits` does not fill the storage",
    label = "`Bits` is less than the bits of the storage",
    note = "`[u64; N]` holds `64 * N` bits",
    note = "`checked_from_limbs` takes the limbs of any width that fits and checks the value"
  )]
  pub trait ReprFilled {}

  impl ReprFilled for typenum::B1 {}

  /// Seals [`UintRepr`](super::UintRepr) and gives the trait implementations of
  /// [`Uint`](crate::Uint) access to the integer core.
  ///
  /// A bound `R: UintRepr` makes these methods callable from any crate, so each takes a [`Token`].
  /// They rely on the width invariant of `Uint`: the representation, read as an unsigned integer,
  /// equals the value.
  pub trait SealedUintRepr: Sized {
    /// Zero.
    fn zero(token: Token) -> Self;

    /// Compares the values of `self` and `other`.
    fn compare(&self, other: &Self, token: Token) -> Ordering;

    /// Writes the lowest `ceil(bits / 64)` 64-bit limbs of the value to `state`, least significant
    /// first. Requires the value to have `bits` bits.
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token);

    /// Writes the value in decimal, honoring the flags of `f` as integers do.
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;
  }

  /// Seals [`IntRepr`](super::IntRepr) and gives the trait implementations of [`Int`](crate::Int)
  /// access to the integer core.
  ///
  /// As for [`SealedUintRepr`], each method takes a [`Token`]. They rely on the width invariant of
  /// `Int`: the representation, read as a two's complement integer, equals the value.
  pub trait SealedIntRepr: Sized {
    /// Zero.
    fn zero(token: Token) -> Self;

    /// Compares the values of `self` and `other`.
    fn compare(&self, other: &Self, token: Token) -> Ordering;

    /// Writes the lowest `ceil(bits / 64)` 64-bit limbs of the value in two's complement to
    /// `state`, least significant first. Requires the value to have `bits` bits.
    fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, token: Token);

    /// Writes the value in decimal, honoring the flags of `f` as integers do.
    fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, token: Token) -> fmt::Result;
  }
}

/// Marks that `Repr` holds every value of `Self` bits.
///
/// `Self` is a [`typenum`] number of at least 1. The array `[u64; N]` holds up to `64 * N` bits,
/// for every `N` for which `typenum` implements `ToUInt` for `Const<N>`: each `N` up to 1024, and
/// larger ones such as the powers of 2 and of 10. A storage may have more bits than needed, as
/// `[u64; 4]` has for 200 bits. [`Uint`](crate::Uint) and [`Int`](crate::Int) require this bound
/// on their constants, constructors and `Default`, not on the types themselves, so generic code
/// adds it where it uses them:
///
/// ```
/// use numo::{FitsBits, Uint};
///
/// fn largest<B: FitsBits<[u64; 2]>>() -> Uint<B, [u64; 2]> {
///   Uint::<B, [u64; 2]>::MAX
/// }
///
/// assert_eq!(largest::<numo::typenum::U100>().to_string(), "1267650600228229401496703205375");
/// ```
///
/// This trait is sealed and cannot be implemented outside this crate.
// The compiler shows this text to a generic caller whose bounds do not prove the relation; for a
// concrete width, it shows the text of the comparison instead.
#[diagnostic::on_unimplemented(
  message = "the storage cannot hold `Bits` bits",
  label = "`Bits` is not known to fit in the storage",
  note = "add a `FitsBits<Repr>` bound on `Bits`"
)]
pub trait FitsBits<Repr>: Unsigned + NonZero + SealedFitsBits<Repr> {}

impl<B, R> FitsBits<R> for B where B: Unsigned + NonZero + SealedFitsBits<R> {}

/// Marks that `Self` bits fill `Repr` exactly, so that every bit pattern of `Repr` is a value.
///
/// `64 * N` bits fill the array `[u64; N]`. The constructor `from_limbs` of
/// [`Uint`](crate::Uint::from_limbs) and [`Int`](crate::Int::from_limbs) requires this bound and
/// never fails; `checked_from_limbs` takes the limbs of any width that fits and checks the value.
/// This trait is sealed and cannot be implemented outside this crate.
#[diagnostic::on_unimplemented(
  message = "`Bits` must fill the storage exactly",
  label = "`Bits` is not known to fill the storage",
  note = "add a `FillsRepr<Repr>` bound on `Bits`, or use `checked_from_limbs`, which checks the value"
)]
pub trait FillsRepr<Repr>: FitsBits<Repr> + SealedFillsRepr<Repr> {}

impl<B, R> FillsRepr<R> for B where B: FitsBits<R> + SealedFillsRepr<R> {}

/// `64 * N`, the number of bits of `[u64; N]`.
type LimbBits<const N: usize> = Shleft<U<N>, U6>;

impl<B, const N: usize> SealedFitsBits<[u64; N]> for B
where
  B: IsLessOrEqual<LimbBits<N>>,
  Const<N>: ToUInt,
  U<N>: Shl<U6>,
  LeEq<B, LimbBits<N>>: BitsFit,
{
}

// `typenum`'s alias for `IsEqual::Output` is named `Eq`, as the trait of `core` is.
impl<B, const N: usize> SealedFillsRepr<[u64; N]> for B
where
  B: IsEqual<LimbBits<N>>,
  Const<N>: ToUInt,
  U<N>: Shl<U6>,
  <B as IsEqual<LimbBits<N>>>::Output: ReprFilled,
{
}

/// A storage of [`Uint`](crate::Uint): the array `[u64; N]` holds an unsigned integer in `N`
/// 64-bit limbs, least significant first.
///
/// `Uint<Bits, Repr>` implements `Ord`, `Hash`, `Display`, `Debug` and `Default` for every `Repr`
/// that implements this trait. This trait is sealed and cannot be implemented outside this crate.
pub trait UintRepr: Eq + SealedUintRepr {}

/// A storage of [`Int`](crate::Int): the array `[u64; N]` holds a signed integer in `N` 64-bit
/// limbs, least significant first, in two's complement.
///
/// `Int<Bits, Repr>` implements `Ord`, `Hash`, `Display`, `Debug` and `Default` for every `Repr`
/// that implements this trait. This trait is sealed and cannot be implemented outside this crate.
pub trait IntRepr: Eq + SealedIntRepr {}

impl<const N: usize> UintRepr for [u64; N] {}

impl<const N: usize> SealedUintRepr for [u64; N] {
  #[inline]
  fn zero(_: Token) -> Self {
    [0; N]
  }

  #[inline]
  fn compare(&self, other: &Self, _: Token) -> Ordering {
    limbs::cmp(self, other)
  }

  #[inline]
  fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
    hash_low_limbs(self, bits, state);
  }

  #[inline]
  fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
    fmt_magnitude(self, true, f)
  }
}

impl<const N: usize> IntRepr for [u64; N] {}

impl<const N: usize> SealedIntRepr for [u64; N] {
  #[inline]
  fn zero(_: Token) -> Self {
    [0; N]
  }

  #[inline]
  fn compare(&self, other: &Self, _: Token) -> Ordering {
    signed_limbs::cmp(self, other)
  }

  #[inline]
  fn hash_limbs<H: Hasher>(&self, bits: u32, state: &mut H, _: Token) {
    // The limbs above the sign bit hold its copies, so the lowest limbs are the two's complement
    // of the value in that many limbs.
    hash_low_limbs(self, bits, state);
  }

  #[inline]
  fn fmt_decimal(&self, f: &mut fmt::Formatter<'_>, _: Token) -> fmt::Result {
    fmt_magnitude(
      &signed_limbs::unsigned_abs(self),
      !signed_limbs::is_negative(self),
      f,
    )
  }
}

/// `Bits` as a `u32`.
///
/// `typenum` computes `U32` modulo `2^32` and `U64` modulo `2^64`. A width from `2^32` to
/// `2^64 - 1` bits, which takes an array of half a gibibyte or more, fails to compile here instead
/// of wrapping.
pub(crate) const fn bit_count<B: Unsigned>() -> u32 {
  assert!(B::U64 == B::U32 as u64, "`Bits` must be below 2^32");
  B::U32
}

/// Writes the lowest `ceil(bits / 64)` limbs of `value` to `state`, least significant first,
/// without the length prefix of a slice.
fn hash_low_limbs<H: Hasher>(value: &[u64], bits: u32, state: &mut H) {
  for &limb in &value[..bits.div_ceil(64) as usize] {
    state.write_u64(limb);
  }
}

/// Writes `magnitude` in decimal, negative unless `is_nonnegative`, honoring the flags of `f` as
/// integers do.
fn fmt_magnitude<const N: usize>(
  magnitude: &[u64; N],
  is_nonnegative: bool,
  f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
  // `2^64 < 10^20`, so each limb adds at most 20 digits.
  let mut buf = [[0; 20]; N];
  let buf = buf.as_flattened_mut();
  let start = limbs::write_digits_into(magnitude, buf);
  f.pad_integral(is_nonnegative, "", ascii(&buf[start..])?)
}

/// Views digits produced by the integer core, which are always ASCII, as a string.
const fn ascii(digits: &[u8]) -> Result<&str, fmt::Error> {
  match core::str::from_utf8(digits) {
    Ok(text) => Ok(text),
    Err(_) => Err(fmt::Error),
  }
}
