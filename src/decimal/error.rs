use core::fmt;

/// The error returned when parsing a [`Decimal`](crate::Decimal) fails.
///
/// The error does not contain the input; [`kind`](Self::kind) tells why parsing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParseError {
  kind: ParseErrorKind,
}

impl ParseError {
  pub(crate) const fn new(kind: ParseErrorKind) -> Self {
    Self { kind }
  }

  /// Returns why parsing failed.
  #[must_use]
  pub const fn kind(&self) -> ParseErrorKind {
    self.kind
  }
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self.kind {
      ParseErrorKind::Empty => "cannot parse decimal from empty string",
      ParseErrorKind::InvalidSyntax => "invalid decimal syntax",
      ParseErrorKind::OutOfRange => "decimal out of range of the target type",
      ParseErrorKind::Inexact => "decimal cannot be represented exactly in the target type",
    })
  }
}

impl core::error::Error for ParseError {}

/// Why parsing a [`Decimal`](crate::Decimal) failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParseErrorKind {
  /// The input is empty.
  Empty,
  /// The input does not match `["-"] 1*DIGIT ["." 1*DIGIT]`.
  InvalidSyntax,
  /// The integral part, without leading zeros, has more than `Integral` digits.
  OutOfRange,
  /// A fraction digit beyond the first `Fraction` is not zero.
  Inexact,
}

/// The error returned by [`Decimal::try_convert`](crate::Decimal::try_convert).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConvertError {
  /// The value is outside the range of the target type.
  OutOfRange,
  /// The target type has fewer fraction digits, and a dropped digit is not zero.
  Inexact,
}

impl fmt::Display for ConvertError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::OutOfRange => "decimal out of range of the target type",
      Self::Inexact => "decimal cannot be represented exactly in the target type",
    })
  }
}

impl core::error::Error for ConvertError {}
