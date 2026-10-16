//! Exact fixed-point decimal numbers whose digit counts are part of the type.
//!
//! [`Decimal<Integral, Fraction, Repr>`](Decimal) stores the value `c / 10^Fraction` as an integer
//! coefficient `c` of type `Repr`, with `|c| <= 10^(Integral + Fraction) - 1`. The digit counts
//! are [`typenum`] numbers, so capacity is checked during type checking: no value can be created
//! for a type with more than 38 digits or with a `Repr` that cannot hold its digits. `Repr`
//! defaults to the smallest signed integer that holds the digits; an unsigned `Repr`, such as
//! `u64` for up to 19 digits, gives a type whose values are never negative.
//!
//! ```
//! use arbitrary_numbers::{
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
//! # Ok::<(), arbitrary_numbers::ParseError>(())
//! ```
//!
//! Every operation is exact: it returns the exact result or reports failure, and nothing rounds,
//! truncates or wraps. Parsing accepts decimal and exponent notation with `_` separators,
//! including every form that `f64` parses, and rejects infinity, NaN and digits that do not fit;
//! formatting writes the shortest exact text. See [`Decimal`] for the details.
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
#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![deny(missing_docs)]

#[cfg(test)]
extern crate std;

pub use typenum;

pub use decimal::{ConvertError, Decimal, ParseError, ParseErrorKind};
pub use layout::{DigitCount, FitsDigits, Layout, Pick, Storage, Widen, WidenRepr};

mod decimal;
mod int;
mod layout;
