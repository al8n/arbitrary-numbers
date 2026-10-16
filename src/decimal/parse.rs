use super::{ParseError, ParseErrorKind};

/// Positions of the digits that a valid input contributes to the coefficient.
pub(super) struct Digits {
  /// Whether the input starts with `-`.
  pub(super) negative: bool,
  /// Start of the integral digits, after any leading zeros.
  pub(super) integral_start: usize,
  /// End of the integral digits.
  pub(super) integral_end: usize,
  /// Start of the fraction digits.
  pub(super) fraction_start: usize,
  /// End of the fraction digits that are kept: at most `fraction` of them.
  pub(super) fraction_end: usize,
  /// Number of zeros to append so that the coefficient has exactly `fraction` fraction digits.
  pub(super) padding: u32,
}

/// Validates `input` for a decimal with `integral` integral and `fraction` fraction digits, and
/// a sign only if `signed`.
///
/// Errors are reported in priority order: `Empty`; `InvalidSyntax` if any byte breaks
/// `["-"] 1*DIGIT ["." 1*DIGIT]`, or if the input starts with `-` and `signed` is `false`;
/// `OutOfRange` if the integral part has more than `integral` digits after its leading zeros;
/// `Inexact` if a fraction digit after the first `fraction` is not `0`. Runs in linear time and
/// constant space.
pub(super) const fn scan(
  input: &[u8],
  integral: u32,
  fraction: u32,
  signed: bool,
) -> Result<Digits, ParseError> {
  let len = input.len();
  if len == 0 {
    return Err(ParseError::new(ParseErrorKind::Empty));
  }

  let negative = input[0] == b'-';
  if negative && !signed {
    return Err(ParseError::new(ParseErrorKind::InvalidSyntax));
  }
  let mut pos = if negative { 1 } else { 0 };

  let digits_start = pos;
  while pos < len && input[pos].is_ascii_digit() {
    pos += 1;
  }
  let integral_end = pos;
  if integral_end == digits_start {
    return Err(ParseError::new(ParseErrorKind::InvalidSyntax));
  }

  let mut fraction_start = pos;
  let mut fraction_end = pos;
  if pos < len {
    if input[pos] != b'.' {
      return Err(ParseError::new(ParseErrorKind::InvalidSyntax));
    }
    pos += 1;
    fraction_start = pos;
    while pos < len && input[pos].is_ascii_digit() {
      pos += 1;
    }
    fraction_end = pos;
    if fraction_end == fraction_start || pos < len {
      return Err(ParseError::new(ParseErrorKind::InvalidSyntax));
    }
  }

  let mut integral_start = digits_start;
  while integral_start < integral_end && input[integral_start] == b'0' {
    integral_start += 1;
  }
  if integral_end - integral_start > integral as usize {
    return Err(ParseError::new(ParseErrorKind::OutOfRange));
  }

  let mut kept_end = fraction_end;
  if fraction_end - fraction_start > fraction as usize {
    kept_end = fraction_start + fraction as usize;
    let mut i = kept_end;
    while i < fraction_end {
      if input[i] != b'0' {
        return Err(ParseError::new(ParseErrorKind::Inexact));
      }
      i += 1;
    }
  }

  Ok(Digits {
    negative,
    integral_start,
    integral_end,
    fraction_start,
    fraction_end: kept_end,
    padding: fraction - (kept_end - fraction_start) as u32,
  })
}
