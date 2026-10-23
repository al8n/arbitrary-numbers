//! Exact fixed-point decimal numbers whose digit counts are part of the type.
//!
//! [`Decimal<Integral, Fraction, Repr>`](Decimal) stores the value `c / 10^Fraction` as an integer
//! coefficient `c` of type `Repr`, with `|c| <= 10^(Integral + Fraction) - 1`. The digit counts
//! are [`typenum`] numbers, so capacity is checked during type checking: no value can be created
//! for a type with more than 154 digits or with a `Repr` that cannot hold its digits. `Repr`
//! defaults to the smallest signed integer that holds the digits, up to 153 digits; an unsigned
//! `Repr`, such as `u64` for up to 19 digits, gives a type whose values are never negative.
//!
//! ```
//! use numo::{
//!   Decimal,
//!   typenum::{U8, U10, U12},
//! };
//!
//! type Price = Decimal<U10, U8>; // 18 digits, stored in an `i64`
//! type Quantity = Decimal<U12, U8>; // 20 digits, stored in an `i128`
//!
//! let price: Price = "64250.5".parse()?;
//! let quantity: Quantity = "0.0015".parse()?;
//!
//! // The product type, `Decimal<U22, U16>`, is computed by the compiler; nothing is rounded.
//! let notional = price.exact_mul(quantity);
//! assert_eq!(notional.to_string(), "96.37575");
//!
//! // Sums are checked against the decimal range, not only the storage range.
//! assert_eq!(Price::MAX.checked_add(price), None);
//! # Ok::<(), numo::ParseError>(())
//! ```
//!
//! Every operation is exact: it returns the exact result or reports failure, and nothing rounds,
//! truncates or wraps unless asked to. Rounding takes an explicit [`RoundingMode`]:
//! [`round_to`](Decimal::round_to) converts to fewer fraction digits,
//! [`quantize`](Decimal::quantize) rounds to a multiple of a step and
//! [`div_round`](Decimal::div_round) divides into any decimal type, while
//! [`is_multiple_of`](Decimal::is_multiple_of) checks a value against a step without rounding it.
//! Parsing accepts decimal and exponent notation with `_` separators, including every form that
//! `f64` parses, and rejects infinity, NaN and digits that do not fit; formatting writes the
//! shortest exact text. See [`Decimal`] for the details.
//!
//! # Wide storage
//!
//! Up to 38 digits, `Repr` is a native integer. Beyond that, up to 153 digits, it is one of the
//! wide integers of this crate, aliases of [`Uint`] and [`Int`] that store 64-bit limbs: the signed
//! [`i192`], [`i256`], [`i320`], [`i384`], [`i448`] and [`i512`] hold 57, 76, 96, 115, 134 and 153
//! digits, and the unsigned [`u192`], [`u256`], [`u320`], [`u384`], [`u448`] and [`u512`] hold 57,
//! 77, 96, 115, 134 and 154. So any uint256 amount, which has at most 78 digits, fits a `u320`. The
//! 154th digit is available only with an explicit `u512`, as in `Decimal<U154, U0, u512>`; the
//! default storage and the product of [`exact_mul`](Decimal::exact_mul) stop at 153 digits, and a
//! product with more digits fails to compile.
//!
//! A wide coefficient is built from its limbs, least significant first, and
//! [`from_repr`](Decimal::from_repr) returns `None` when it is out of range. Parsing decimal text
//! with an exponent gives the same value:
//!
//! ```
//! use numo::{
//!   Decimal,
//!   typenum::{U18, U60},
//!   u320,
//! };
//!
//! // Any uint256 amount of a token with 18 decimals.
//! type TokenAmount = Decimal<U60, U18, u320>;
//!
//! // The raw amount 2^64 + 5 as the four limbs of a uint256, with a fifth limb of zero.
//! let raw: [u64; 4] = [5, 1, 0, 0];
//! let [l0, l1, l2, l3] = raw;
//! let amount = TokenAmount::from_repr(u320::from_limbs([l0, l1, l2, l3, 0])).unwrap();
//! assert_eq!(amount.to_string(), "18.446744073709551621");
//! assert_eq!("18446744073709551621e-18".parse::<TokenAmount>()?, amount);
//! # Ok::<(), numo::ParseError>(())
//! ```
//!
//! The wide integers provide only what a storage needs: [`from_limbs`](u256::from_limbs),
//! [`to_limbs`](u256::to_limbs), `ZERO`, numeric equality, ordering and hashing, and decimal
//! formatting. They have no arithmetic of their own.
//!
//! # `typenum`
//!
//! The crate re-exports [`typenum`], so no direct dependency is needed. Digit counts are written
//! as its constants, such as `typenum::U8`, or as `typenum::U<8>`. The constants are not
//! re-exported at the crate root, which keeps the root namespace to this crate's own items.
//!
//! # `no_std`
//!
//! The crate is `no_std` and never allocates.
//!
//! # `serde`
//!
//! The `serde` feature implements `Serialize` and `Deserialize` for [`Decimal`] and
//! [`RoundingMode`]. The impls are written against `serde_core`, whose traits `serde` re-exports
//! from version 1.0.220 on, so they need a build whose `serde` is 1.0.220 or later; an older
//! `serde` defines traits of its own. The feature enables `serde_core` without its default
//! features and nothing else, so it turns on no feature in any other crate of a build, and it
//! needs no allocation.
//!
//! A decimal is written as a string, the text that `Display` writes, and read from a string in the
//! syntax of [`parse`](Decimal::parse), in every format, human-readable or binary. Reading
//! rejects a number, so a decimal never passes through a binary floating-point value, which would
//! change `16.7` to the nearest `f64`. Nor is the coefficient written alone: it depends on the
//! number of fraction digits, and read back into a type with another number it would be off by a
//! power of ten. A rounding mode is written as the name of its variant, such as `"HalfEven"`.
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![deny(missing_docs)]

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod bigint;

pub use typenum;

pub use decimal::{ConvertError, Decimal, DivideError, ParseError, ParseErrorKind, QuantizeError};
pub use integer::{
  Int, Uint, i192, i256, i320, i384, i448, i512, u192, u256, u320, u384, u448, u512,
};
pub use layout::{
  DigitCount, DivIntermediate, FitsDigits, Layout, Pick, Storage, Widen, WidenRepr,
};
pub use rounding::RoundingMode;

mod decimal;
mod int;
mod integer;
mod layout;
mod rounding;
#[cfg(feature = "serde")]
mod serde;
