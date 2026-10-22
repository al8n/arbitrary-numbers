use core::fmt::{self, Alignment, Formatter, Write};

/// Zeros written in chunks, so padding and precision need no buffer of their size.
const ZEROS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The pieces of the text of a decimal, in output order.
struct Parts<'a> {
  sign: Option<char>,
  integral: &'a str,
  /// Zeros between the decimal point and `fraction` that the magnitude digits do not contain, as
  /// in `0.05` for the digits `5`.
  leading_zeros: usize,
  /// Fraction digits from the magnitude, ending at the last nonzero digit.
  fraction: &'a str,
  /// Zeros after `fraction` requested by the precision.
  trailing_zeros: usize,
}

impl Parts<'_> {
  fn fraction_len(&self) -> usize {
    self
      .leading_zeros
      .saturating_add(self.fraction.len())
      .saturating_add(self.trailing_zeros)
  }

  fn len(&self) -> usize {
    let point = match self.fraction_len() {
      0 => 0,
      len => len.saturating_add(1),
    };
    usize::from(self.sign.is_some())
      .saturating_add(self.integral.len())
      .saturating_add(point)
  }

  fn write_sign(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self.sign {
      Some(sign) => f.write_char(sign),
      None => Ok(()),
    }
  }

  fn write_digits(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(self.integral)?;
    if self.fraction_len() > 0 {
      f.write_char('.')?;
      write_zeros(f, self.leading_zeros)?;
      f.write_str(self.fraction)?;
      write_zeros(f, self.trailing_zeros)?;
    }
    Ok(())
  }
}

/// Writes a decimal whose magnitude has the ASCII digits `digits`, without leading zeros (zero is
/// a single `0`), of which the last `fraction` are fraction digits.
///
/// The shortest exact form is padded like an integer: `+` adds a sign to non-negative values, `0`
/// pads with zeros after the sign, and otherwise the fill character pads to the width with the
/// requested alignment, right by default. The precision is the minimum number of fraction digits.
pub(super) fn write(
  f: &mut Formatter<'_>,
  negative: bool,
  digits: &[u8],
  fraction: usize,
) -> fmt::Result {
  let (integral, fraction_digits, leading_zeros) = match digits.len().checked_sub(fraction) {
    Some(integral_len) if integral_len > 0 => {
      let (integral, fraction_digits) = digits.split_at(integral_len);
      (integral, fraction_digits, 0)
    }
    _ => (&b"0"[..], digits, fraction - digits.len()),
  };

  let significant_len = fraction_digits
    .iter()
    .rposition(|&digit| digit != b'0')
    .map_or(0, |last| last + 1);
  let (leading_zeros, fraction_digits) = match significant_len {
    0 => (0, &fraction_digits[..0]),
    len => (leading_zeros, &fraction_digits[..len]),
  };
  let shown = leading_zeros + fraction_digits.len();
  let trailing_zeros = f
    .precision()
    .map_or(0, |precision| precision.saturating_sub(shown));

  let sign = if negative {
    Some('-')
  } else if f.sign_plus() {
    Some('+')
  } else {
    None
  };

  let parts = Parts {
    sign,
    integral: ascii(integral)?,
    leading_zeros,
    fraction: ascii(fraction_digits)?,
    trailing_zeros,
  };

  let padding = match f.width() {
    Some(width) => width.saturating_sub(parts.len()),
    None => 0,
  };
  if padding == 0 {
    parts.write_sign(f)?;
    return parts.write_digits(f);
  }

  if f.sign_aware_zero_pad() {
    parts.write_sign(f)?;
    write_zeros(f, padding)?;
    return parts.write_digits(f);
  }

  let (before, after) = match f.align() {
    Some(Alignment::Left) => (0, padding),
    Some(Alignment::Center) => (padding / 2, padding - padding / 2),
    Some(Alignment::Right) | None => (padding, 0),
  };
  let fill = f.fill();
  write_fill(f, fill, before)?;
  parts.write_sign(f)?;
  parts.write_digits(f)?;
  write_fill(f, fill, after)
}

/// Views digits produced by the integer core, which are always ASCII, as a string.
fn ascii(digits: &[u8]) -> Result<&str, fmt::Error> {
  core::str::from_utf8(digits).map_err(|_| fmt::Error)
}

fn write_zeros(f: &mut Formatter<'_>, mut count: usize) -> fmt::Result {
  while count > 0 {
    let chunk = count.min(ZEROS.len());
    f.write_str(&ZEROS[..chunk])?;
    count -= chunk;
  }
  Ok(())
}

fn write_fill(f: &mut Formatter<'_>, fill: char, count: usize) -> fmt::Result {
  for _ in 0..count {
    f.write_char(fill)?;
  }
  Ok(())
}
