//! Integers whose width is a type-level number of bits.

use core::{
  cmp::Ordering,
  fmt,
  hash::{Hash, Hasher},
  marker::PhantomData,
};

use typenum::{U192, U256, U320};

use crate::int::{self, DIGIT_BUF_LEN};

/// An unsigned integer of `Bits` bits, stored in `Repr`.
///
/// The value runs from zero to `2^Bits - 1`. `Bits` is a [`typenum`] number. The aliases
/// [`u192`], [`u256`] and [`u320`] store their value in arrays of 64-bit limbs, least significant
/// first, and are the widths provided. Each has [`ZERO`](u256::ZERO),
/// [`from_limbs`](u256::from_limbs) and [`to_limbs`](u256::to_limbs), numeric equality, ordering
/// and hashing, and decimal formatting; there is no arithmetic. The type has the size, alignment
/// and ABI of `Repr`.
///
/// These integers are the wide storages of [`Decimal`](crate::Decimal):
///
/// ```
/// use arbitrary_numbers::{
///   Decimal,
///   typenum::{U18, U59},
///   u256,
/// };
///
/// // 2^64 + 5, least significant limb first.
/// let value = u256::from_limbs([5, 1, 0, 0]);
/// assert_eq!(value.to_limbs(), [5, 1, 0, 0]);
/// assert_eq!(value.to_string(), "18446744073709551621");
/// assert!(u256::ZERO < value);
///
/// let max = u256::from_limbs([u64::MAX; 4]);
/// assert_eq!(
///   max.to_string(),
///   "115792089237316195423570985008687907853269984665640564039457584007913129639935"
/// );
///
/// // 2^64 + 5 units of 10^-18.
/// let amount = Decimal::<U59, U18, u256>::from_repr(value).unwrap();
/// assert_eq!(amount.to_string(), "18.446744073709551621");
/// ```
///
/// The width cannot be inferred from the limbs, so construction names an alias:
/// `Uint::from_limbs(limbs)` does not compile.
#[repr(transparent)]
pub struct Uint<Bits, Repr> {
  repr: Repr,
  bits: PhantomData<Bits>,
}

/// A signed integer of `Bits` bits in two's complement, stored in `Repr`.
///
/// The value runs from `-2^(Bits - 1)` to `2^(Bits - 1) - 1`. `Bits` is a [`typenum`] number. The
/// aliases [`i192`], [`i256`] and [`i320`] store their value in arrays of 64-bit limbs, least
/// significant first, and are the widths provided. Each has [`ZERO`](i256::ZERO),
/// [`from_limbs`](i256::from_limbs) and [`to_limbs`](i256::to_limbs), numeric equality, ordering
/// and hashing, and decimal formatting; there is no arithmetic. The type has the size, alignment
/// and ABI of `Repr`.
///
/// ```
/// use arbitrary_numbers::i256;
///
/// let minus_one = i256::from_limbs([u64::MAX; 4]);
/// assert_eq!(minus_one.to_string(), "-1");
/// assert_eq!(format!("{minus_one:>4}"), "  -1");
///
/// // Ordering is numeric: the most negative value has only the sign bit set.
/// let min = i256::from_limbs([0, 0, 0, 1 << 63]);
/// assert!(min < minus_one && minus_one < i256::ZERO);
/// assert_eq!(
///   min.to_string(),
///   "-57896044618658097711785492504343953926634992332820282019728792003956564819968"
/// );
/// ```
///
/// The width cannot be inferred from the limbs, so construction names an alias:
/// `Int::from_limbs(limbs)` does not compile.
#[repr(transparent)]
pub struct Int<Bits, Repr> {
  repr: Repr,
  bits: PhantomData<Bits>,
}

/// A 192-bit unsigned integer, stored in three 64-bit limbs.
#[allow(non_camel_case_types)]
pub type u192 = Uint<U192, [u64; 3]>;

/// A 256-bit unsigned integer, stored in four 64-bit limbs.
#[allow(non_camel_case_types)]
pub type u256 = Uint<U256, [u64; 4]>;

/// A 320-bit unsigned integer, stored in five 64-bit limbs.
#[allow(non_camel_case_types)]
pub type u320 = Uint<U320, [u64; 5]>;

/// A 192-bit signed integer, stored in three 64-bit limbs in two's complement.
#[allow(non_camel_case_types)]
pub type i192 = Int<U192, [u64; 3]>;

/// A 256-bit signed integer, stored in four 64-bit limbs in two's complement.
#[allow(non_camel_case_types)]
pub type i256 = Int<U256, [u64; 4]>;

/// A 320-bit signed integer, stored in five 64-bit limbs in two's complement.
#[allow(non_camel_case_types)]
pub type i320 = Int<U320, [u64; 5]>;

// The API is implemented for each alias rather than for every `Bits`: a width that does not fill
// its limbs, such as 200 bits in `[u64; 4]`, has bit patterns that are not values, so its
// constructor must check them.
macro_rules! wide_integer {
  (unsigned $name:ident: $limbs:literal) => {
    impl $name {
      wide_integer!(@constructors $name, $limbs, "");
    }

    wide_integer!(@traits $name);

    impl fmt::Display for $name {
      /// Writes the value in decimal, honoring width, fill, alignment, `+` and `0` as integers do.
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0; DIGIT_BUF_LEN];
        let start = int::$name::write_digits(*self, &mut buf);
        f.pad_integral(true, "", ascii(&buf[start..])?)
      }
    }
  };
  (signed $name:ident: $limbs:literal) => {
    impl $name {
      wide_integer!(@constructors $name, $limbs, ", in two's complement");
    }

    wide_integer!(@traits $name);

    impl fmt::Display for $name {
      /// Writes the value in decimal, honoring width, fill, alignment, `+` and `0` as integers do.
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0; DIGIT_BUF_LEN];
        let start = int::$name::write_digits(*self, &mut buf);
        f.pad_integral(!int::$name::is_negative(*self), "", ascii(&buf[start..])?)
      }
    }
  };
  (@constructors $name:ident, $limbs:literal, $encoding:literal) => {
    /// Zero.
    pub const ZERO: Self = Self::from_limbs([0; $limbs]);

    #[doc = concat!(
      "Creates an integer from its 64-bit limbs, least significant first",
      $encoding,
      "."
    )]
    ///
    /// Every bit pattern is a value, so this never fails.
    #[must_use]
    #[inline]
    pub const fn from_limbs(limbs: [u64; $limbs]) -> Self {
      Self {
        repr: limbs,
        bits: PhantomData,
      }
    }

    #[doc = concat!(
      "Returns the 64-bit limbs of the value, least significant first",
      $encoding,
      "."
    )]
    #[must_use]
    #[inline]
    pub const fn to_limbs(self) -> [u64; $limbs] {
      self.repr
    }
  };
  // Trait methods cannot be `const`.
  (@traits $name:ident) => {
    impl Clone for $name {
      #[inline]
      fn clone(&self) -> Self {
        *self
      }
    }

    impl Copy for $name {}

    impl PartialEq for $name {
      #[inline]
      fn eq(&self, other: &Self) -> bool {
        self.repr == other.repr
      }
    }

    impl Eq for $name {}

    impl PartialOrd for $name {
      #[inline]
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
      }
    }

    impl Ord for $name {
      /// Compares the values, from the most significant limb down. The array order would compare
      /// the least significant limb first.
      #[inline]
      fn cmp(&self, other: &Self) -> Ordering {
        int::$name::cmp(*self, *other)
      }
    }

    impl Hash for $name {
      /// Writes each 64-bit limb, least significant first, without the length prefix of an array,
      /// so that the hash depends only on the value and its width.
      fn hash<H: Hasher>(&self, state: &mut H) {
        for limb in self.repr {
          state.write_u64(limb);
        }
      }
    }

    impl fmt::Debug for $name {
      /// Writes the same text as [`Display`](fmt::Display).
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
      }
    }
  };
}

wide_integer!(unsigned u192: 3);
wide_integer!(unsigned u256: 4);
wide_integer!(unsigned u320: 5);

wide_integer!(signed i192: 3);
wide_integer!(signed i256: 4);
wide_integer!(signed i320: 5);

/// Views digits produced by the integer core, which are always ASCII, as a string.
const fn ascii(digits: &[u8]) -> Result<&str, fmt::Error> {
  match core::str::from_utf8(digits) {
    Ok(text) => Ok(text),
    Err(_) => Err(fmt::Error),
  }
}

#[cfg(test)]
mod tests;
