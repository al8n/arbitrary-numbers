//! Integers whose width is a type-level number of bits.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  marker::PhantomData,
};

use crate::{
  int,
  layout::Token,
  width::{FillsRepr, FitsBits, IntRepr, UintRepr},
};

/// An unsigned integer of `Bits` bits, stored in `Repr`.
///
/// The value runs from zero to `2^Bits - 1`. `Bits` is a [`typenum`] number of at least 1, and
/// `Repr` is an array `[u64; N]` of 64-bit limbs, least significant first, that holds at least
/// `Bits` bits; the bits above `Bits`, if any, are zero. The module [`aliases`](crate::aliases)
/// names common widths, such as [`u256`](crate::u256) for `Uint<U256, [u64; 4]>`. The type has the
/// size, alignment and ABI of `Repr`.
///
/// [`from_limbs`](Self::from_limbs) creates a value from any bit pattern of a width that fills its
/// limbs, and [`checked_from_limbs`](Self::checked_from_limbs) from the limbs of any width, checking
/// that they hold a value of `Bits` bits. Values compare, hash and format by their numeric value:
/// `Ord` is numeric, `Hash` depends only on the value and `Bits`, and `Display` and `Debug` write
/// decimal. There is no arithmetic.
///
/// The aliases from 192 to 512 bits are the wide storages of [`Decimal`](crate::Decimal):
///
/// ```
/// use numo::{
///   Decimal, Uint,
///   typenum::{U18, U59, U200},
///   u256,
/// };
///
/// // 2^64 + 5, least significant limb first.
/// let value = u256::from_limbs([5, 1, 0, 0]);
/// assert_eq!(value.to_limbs(), [5, 1, 0, 0]);
/// assert_eq!(value.to_string(), "18446744073709551621");
/// assert!(u256::ZERO < value && value < u256::MAX);
/// assert_eq!(
///   u256::MAX.to_string(),
///   "115792089237316195423570985008687907853269984665640564039457584007913129639935"
/// );
///
/// // 200 bits in four limbs: the top 56 bits of the last limb are zero.
/// type U200x4 = Uint<U200, [u64; 4]>;
/// assert_eq!(U200x4::MAX.to_limbs(), [u64::MAX, u64::MAX, u64::MAX, 0xff]);
/// assert_eq!(U200x4::checked_from_limbs([5, 1, 0, 0]).unwrap().to_string(), value.to_string());
/// assert_eq!(U200x4::checked_from_limbs([0, 0, 0, 0x100]), None);
///
/// // 2^64 + 5 units of 10^-18.
/// let amount = Decimal::<U59, U18, u256>::from_repr(value).unwrap();
/// assert_eq!(amount.to_string(), "18.446744073709551621");
/// ```
///
/// The width cannot be inferred from the limbs, so construction names the alias or the full type,
/// for example `u256::from_limbs(limbs)` or `Uint::<U200, [u64; 4]>::checked_from_limbs(limbs)`.
#[repr(transparent)]
pub struct Uint<Bits, Repr> {
  // Holds a value of `Bits` bits: every bit of the limbs at index `Bits` or above is zero, so the
  // limbs, read as an unsigned integer, equal the value. Every constructor establishes this, and
  // equality, comparison, hashing and formatting rely on it. It is a logical invariant, not a
  // safety invariant: no `unsafe` code may rely on it.
  repr: Repr,
  bits: PhantomData<Bits>,
}

/// A signed integer of `Bits` bits in two's complement, stored in `Repr`.
///
/// The value runs from `-2^(Bits - 1)` to `2^(Bits - 1) - 1`. `Bits` is a [`typenum`] number of at
/// least 1, and `Repr` is an array `[u64; N]` of 64-bit limbs, least significant first, that holds
/// at least `Bits` bits; the bits above `Bits`, if any, are copies of the sign bit, so the limbs
/// hold the value in two's complement. The module [`aliases`](crate::aliases) names common widths,
/// such as [`i256`](crate::i256) for `Int<U256, [u64; 4]>`. The type has the size, alignment and
/// ABI of `Repr`.
///
/// As for [`Uint`], [`from_limbs`](Self::from_limbs) creates a value from any bit pattern of a width
/// that fills its limbs, and [`checked_from_limbs`](Self::checked_from_limbs) from the limbs of any
/// width, checking that they hold a value of `Bits` bits. Values compare, hash and format by their
/// numeric value, and there is no arithmetic.
///
/// ```
/// use numo::{
///   Int, i256,
///   typenum::{U7, U100},
/// };
///
/// let minus_one = i256::from_limbs([u64::MAX; 4]);
/// assert_eq!(minus_one.to_string(), "-1");
/// assert_eq!(format!("{minus_one:>4}"), "  -1");
/// assert!(minus_one.is_negative());
///
/// // Ordering is numeric: the most negative value has only the sign bit set.
/// assert_eq!(i256::MIN.to_limbs(), [0, 0, 0, 1 << 63]);
/// assert!(i256::MIN < minus_one && minus_one < i256::ZERO);
/// assert_eq!(
///   i256::MIN.to_string(),
///   "-57896044618658097711785492504343953926634992332820282019728792003956564819968"
/// );
///
/// // 7 bits in one limb: the value runs from -64 to 63, and -1 has every bit set.
/// type I7 = Int<U7, [u64; 1]>;
/// assert_eq!(I7::MIN.to_string(), "-64");
/// assert_eq!(I7::MAX.to_string(), "63");
/// assert_eq!(I7::checked_from_limbs([u64::MAX]).unwrap().to_string(), "-1");
/// // The limb 0x7f holds 127, which needs 8 bits.
/// assert_eq!(I7::checked_from_limbs([0x7f]), None);
///
/// // 100 bits in two limbs: the 28 bits above the sign bit copy it.
/// assert_eq!(Int::<U100, [u64; 2]>::MIN.to_limbs(), [0, 0xffff_fff8 << 32]);
/// ```
///
/// The width cannot be inferred from the limbs, so construction names the alias or the full type,
/// for example `i256::from_limbs(limbs)` or `Int::<U200, [u64; 4]>::checked_from_limbs(limbs)`.
#[repr(transparent)]
pub struct Int<Bits, Repr> {
  // Holds a value of `Bits` bits: every bit of the limbs at index `Bits - 1` or above equals bit
  // `Bits - 1`, the sign bit, so the limbs, read as a two's complement integer, equal the value.
  // Every constructor establishes this, and equality, comparison, the sign queries, hashing and
  // formatting rely on it. It is a logical invariant, not a safety invariant: no `unsafe` code may
  // rely on it.
  repr: Repr,
  bits: PhantomData<Bits>,
}

impl<B, R> Uint<B, R> {
  /// Wraps `repr`. Requires it to hold a value of `Bits` bits.
  #[inline]
  const fn wrap(repr: R) -> Self {
    Self {
      repr,
      bits: PhantomData,
    }
  }
}

impl<B, R> Int<B, R> {
  /// Wraps `repr`. Requires it to hold a value of `Bits` bits.
  #[inline]
  const fn wrap(repr: R) -> Self {
    Self {
      repr,
      bits: PhantomData,
    }
  }
}

/// Generates the constants and the `const fn`s of [`Uint`] and [`Int`] over the arrays of each limb
/// type, written `limb, bits, lower, upper;`, where `bits` is the number of bits of a limb and
/// `lower` and `upper` are the modules of its limb layers, and followed by the examples of
/// `checked_from_limbs` for `Uint` and for `Int`.
// Trait methods cannot be `const`, so each limb type has inherent impls of its own.
macro_rules! array_integers {
  ($(
    $limb:ident, $bits:literal, $lower:ident, $upper:ident;
    Uint: { $(#[$uint_example:meta])* }
    Int: { $(#[$int_example:meta])* }
  )+) => {$(
    impl<B, const N: usize> Uint<B, [$limb; N]>
    where
      B: FitsBits<[$limb; N]>,
    {
      /// The number of bits, `Bits`.
      pub const BITS: u32 = B::U32;

      /// Zero.
      pub const ZERO: Self = Self::wrap([0; N]);

      /// The smallest value, zero.
      pub const MIN: Self = Self::ZERO;

      /// The largest value, `2^Bits - 1`.
      pub const MAX: Self = Self::wrap(int::$lower::max_of_bits(Self::BITS));

      #[doc = concat!(
        "Creates an integer from its ",
        stringify!($bits),
        "-bit limbs, least significant first."
      )]
      ///
      /// `Bits` fills the limbs, so every bit pattern is a value and this never fails. For a width that
      /// leaves bits unused, [`checked_from_limbs`](Self::checked_from_limbs) checks the limbs instead.
      #[must_use]
      #[inline]
      pub const fn from_limbs(limbs: [$limb; N]) -> Self
      where
        B: FillsRepr<[$limb; N]>,
      {
        Self::wrap(limbs)
      }

      // Each line of this documentation starts with a space, as each line of the examples does, so
      // that rustdoc removes one space from every line.
      #[doc = concat!(
        " Creates an integer from its ",
        stringify!($bits),
        "-bit limbs, least significant first, or returns `None` if they"
      )]
      #[doc = " hold `2^Bits` or more: a bit at index `Bits` or above is set."]
      #[doc = ""]
      #[doc = " For a width that fills its limbs, this never returns `None`."]
      #[doc = ""]
      $(#[$uint_example])*
      #[must_use]
      #[inline]
      pub const fn checked_from_limbs(limbs: [$limb; N]) -> Option<Self> {
        if int::$lower::fits_bits(&limbs, Self::BITS) {
          Some(Self::wrap(limbs))
        } else {
          None
        }
      }
    }

    impl<B, const N: usize> Uint<B, [$limb; N]> {
      #[doc = concat!(
        "Returns the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first. The bits above `Bits`, if any,"
      )]
      /// are zero.
      #[must_use]
      #[inline]
      pub const fn to_limbs(self) -> [$limb; N] {
        self.repr
      }

      #[doc = concat!(
        "Borrows the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first. The bits above `Bits`, if any,"
      )]
      /// are zero.
      #[must_use]
      #[inline]
      pub const fn as_limbs(&self) -> &[$limb; N] {
        &self.repr
      }

      /// Returns `true` if the value is zero.
      #[must_use]
      #[inline]
      pub const fn is_zero(&self) -> bool {
        int::$lower::is_zero(&self.repr)
      }
    }

    impl<B, const N: usize> Int<B, [$limb; N]>
    where
      B: FitsBits<[$limb; N]>,
    {
      /// The number of bits, `Bits`, including the sign bit.
      pub const BITS: u32 = B::U32;

      /// Zero.
      pub const ZERO: Self = Self::wrap([0; N]);

      /// The smallest value, `-2^(Bits - 1)`.
      pub const MIN: Self = Self::wrap(int::$upper::min_of_bits(Self::BITS));

      /// The largest value, `2^(Bits - 1) - 1`.
      pub const MAX: Self = Self::wrap(int::$upper::max_of_bits(Self::BITS));

      #[doc = concat!(
        "Creates an integer from its ",
        stringify!($bits),
        "-bit limbs, least significant first, in two's complement."
      )]
      ///
      /// `Bits` fills the limbs, so every bit pattern is a value and this never fails. For a width that
      /// leaves bits unused, [`checked_from_limbs`](Self::checked_from_limbs) checks the limbs instead.
      #[must_use]
      #[inline]
      pub const fn from_limbs(limbs: [$limb; N]) -> Self
      where
        B: FillsRepr<[$limb; N]>,
      {
        Self::wrap(limbs)
      }

      // Each line of this documentation starts with a space, as each line of the examples does, so
      // that rustdoc removes one space from every line.
      #[doc = concat!(
        " Creates an integer from its ",
        stringify!($bits),
        "-bit limbs, least significant first, in two's complement, or"
      )]
      #[doc = concat!(
        " returns `None` if the integer they hold is below `-2^(Bits - 1)` or above ",
        "`2^(Bits - 1) - 1`:"
      )]
      #[doc = " a bit at index `Bits - 1` or above differs from the top bit of the limbs."]
      #[doc = ""]
      #[doc = " For a width that fills its limbs, this never returns `None`."]
      #[doc = ""]
      $(#[$int_example])*
      #[must_use]
      #[inline]
      pub const fn checked_from_limbs(limbs: [$limb; N]) -> Option<Self> {
        if int::$upper::fits_bits(&limbs, Self::BITS) {
          Some(Self::wrap(limbs))
        } else {
          None
        }
      }
    }

    impl<B, const N: usize> Int<B, [$limb; N]> {
      #[doc = concat!(
        "Returns the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first, in two's complement. The bits"
      )]
      /// above `Bits`, if any, are copies of the sign bit.
      #[must_use]
      #[inline]
      pub const fn to_limbs(self) -> [$limb; N] {
        self.repr
      }

      #[doc = concat!(
        "Borrows the ",
        stringify!($bits),
        "-bit limbs of the value, least significant first, in two's complement. The bits"
      )]
      /// above `Bits`, if any, are copies of the sign bit.
      #[must_use]
      #[inline]
      pub const fn as_limbs(&self) -> &[$limb; N] {
        &self.repr
      }

      /// Returns `true` if the value is zero.
      #[must_use]
      #[inline]
      pub const fn is_zero(&self) -> bool {
        int::$upper::is_zero(&self.repr)
      }

      /// Returns `true` if the value is negative.
      #[must_use]
      #[inline]
      pub const fn is_negative(&self) -> bool {
        int::$upper::is_negative(&self.repr)
      }

      /// Returns `true` if the value is positive.
      #[must_use]
      #[inline]
      pub const fn is_positive(&self) -> bool {
        int::$upper::is_positive(&self.repr)
      }
    }
  )+};
}

array_integers! {
  u64, 64, limbs, signed_limbs;
  Uint: {
    /// ```
    /// use numo::{Uint, typenum::U65};
    ///
    /// type U65x2 = Uint<U65, [u64; 2]>;
    ///
    /// // 2^64 + 1 and 2^65 - 1 have 65 bits, and 2^65 has 66.
    /// assert_eq!(
    ///   U65x2::checked_from_limbs([1, 1]).unwrap().to_string(),
    ///   "18446744073709551617"
    /// );
    /// assert_eq!(U65x2::checked_from_limbs([u64::MAX, 1]), Some(U65x2::MAX));
    /// assert_eq!(U65x2::checked_from_limbs([0, 2]), None);
    /// ```
  }
  Int: {
    /// ```
    /// use numo::{Int, typenum::U7};
    ///
    /// type I7 = Int<U7, [u64; 1]>;
    ///
    /// // -1, every bit set, and 63, the largest value.
    /// assert!(I7::checked_from_limbs([u64::MAX]).unwrap().is_negative());
    /// assert_eq!(I7::checked_from_limbs([63]), Some(I7::MAX));
    /// // 127 and -65 need 8 bits.
    /// assert_eq!(I7::checked_from_limbs([127]), None);
    /// assert_eq!(I7::checked_from_limbs([-65i64 as u64]), None);
    /// ```
  }
}

// Trait methods cannot be `const`.
macro_rules! integer_traits {
  ($name:ident, $repr:ident) => {
    impl<B, R> Clone for $name<B, R>
    where
      R: Clone,
    {
      #[inline]
      fn clone(&self) -> Self {
        Self::wrap(self.repr.clone())
      }
    }

    impl<B, R> Copy for $name<B, R> where R: Copy {}

    impl<B, R> PartialEq for $name<B, R>
    where
      R: PartialEq,
    {
      #[inline]
      fn eq(&self, other: &Self) -> bool {
        self.repr == other.repr
      }
    }

    impl<B, R> Eq for $name<B, R> where R: Eq {}

    impl<B, R> PartialOrd for $name<B, R>
    where
      R: $repr,
    {
      #[inline]
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
      }
    }

    impl<B, R> Ord for $name<B, R>
    where
      R: $repr,
    {
      /// Compares the values, from the most significant limb down. The array order would compare
      /// the least significant limb first.
      #[inline]
      fn cmp(&self, other: &Self) -> Ordering {
        self.repr.compare(&other.repr, Token::new())
      }
    }

    impl<B, R> Hash for $name<B, R>
    where
      R: $repr,
      B: FitsBits<R>,
    {
      /// Writes the lowest `ceil(Bits / 64)` 64-bit limbs of the value, least significant first,
      /// without the length prefix of an array, so that the hash depends only on the value and
      /// `Bits`, whatever the number of limbs.
      #[inline]
      fn hash<H: Hasher>(&self, state: &mut H) {
        self.repr.hash_limbs(B::U32, state, Token::new());
      }
    }

    impl<B, R> fmt::Display for $name<B, R>
    where
      R: $repr,
    {
      /// Writes the value in decimal, honoring width, fill, alignment, `+` and `0` as integers do.
      ///
      /// The digits of a `[u64; N]` value take a stack buffer of `20 * N` bytes and time quadratic
      /// in `N`.
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.repr.fmt_decimal(f, Token::new())
      }
    }

    impl<B, R> fmt::Debug for $name<B, R>
    where
      R: $repr,
    {
      /// Writes the same text as [`Display`](fmt::Display).
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
      }
    }

    impl<B, R> Default for $name<B, R>
    where
      R: $repr,
      B: FitsBits<R>,
    {
      /// Returns zero.
      #[inline]
      fn default() -> Self {
        Self::wrap(R::zero(Token::new()))
      }
    }
  };
}

integer_traits!(Uint, UintRepr);
integer_traits!(Int, IntRepr);

#[cfg(test)]
mod tests;
