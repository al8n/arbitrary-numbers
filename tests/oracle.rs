//! Property tests that compare `Decimal` with exact big-integer arithmetic.
//!
//! Coefficients come from three sources: random digit strings biased toward `0` and `9`, random
//! bit patterns scaled to the decimal bound, and the edges of the range. Parsing is checked against
//! an independent implementation of the grammar on numbers generated in every syntactic form, on
//! infinities and NaNs, on sequences of grammar tokens and on random text, and directly against
//! `f64::from_str` on token sequences, rendered `f64` text, `f64`'s documented grammar and near
//! misses of it; every value must read back from its text, also when that text is rewritten with
//! an exponent. Each suite covers one decimal type, with a signed or an unsigned storage.

use arbitrary_numbers::{
  ConvertError, Decimal, ParseErrorKind,
  typenum::{
    U0, U1, U2, U3, U4, U5, U6, U7, U8, U9, U10, U11, U12, U14, U16, U17, U19, U20, U22, U36, U38,
  },
};
use num_bigint::{BigInt, Sign};
use proptest::{prelude::*, test_runner::RngSeed};

fn config() -> ProptestConfig {
  if cfg!(miri) {
    ProptestConfig {
      cases: 4,
      failure_persistence: None,
      rng_seed: RngSeed::Fixed(0x5eed),
      ..ProptestConfig::default()
    }
  } else {
    ProptestConfig::default()
  }
}

fn pow10(exp: u32) -> BigInt {
  BigInt::from(10u8).pow(exp)
}

fn max_coeff(digits: u32) -> BigInt {
  pow10(digits) - 1u8
}

fn in_range(coeff: &BigInt, digits: u32) -> bool {
  coeff.magnitude() <= max_coeff(digits).magnitude()
}

/// The shortest exact text of `coeff / 10^fraction`, padded to at least `precision` fraction
/// digits.
fn oracle_text(coeff: &BigInt, fraction: u32, precision: usize) -> String {
  let fraction = fraction as usize;
  let digits = coeff.magnitude().to_string();
  let digits = if digits.len() <= fraction {
    format!("{}{digits}", "0".repeat(fraction + 1 - digits.len()))
  } else {
    digits
  };
  let (integral, fraction_digits) = digits.split_at(digits.len() - fraction);
  let mut fraction_digits = fraction_digits.trim_end_matches('0').to_owned();
  while fraction_digits.len() < precision {
    fraction_digits.push('0');
  }

  let mut text = String::new();
  if coeff.sign() == Sign::Minus {
    text.push('-');
  }
  text.push_str(integral);
  if !fraction_digits.is_empty() {
    text.push('.');
    text.push_str(&fraction_digits);
  }
  text
}

/// The coefficient that `input` denotes for `integral` and `fraction` digits, where only a signed
/// storage accepts a leading `-`.
///
/// The crate scans the input once, byte by byte, and tracks digit positions. This reference splits
/// the text with string methods instead, and computes the value with big-integer powers of ten.
fn oracle_parse(
  input: &str,
  integral: u32,
  fraction: u32,
  signed: bool,
) -> Result<BigInt, ParseErrorKind> {
  if input.is_empty() {
    return Err(ParseErrorKind::Empty);
  }
  let (negative, body) = match input.strip_prefix('-') {
    Some(_) if !signed => return Err(ParseErrorKind::InvalidSyntax),
    Some(rest) => (true, rest),
    None => (false, input.strip_prefix('+').unwrap_or(input)),
  };

  let lower = body.to_ascii_lowercase();
  if lower == "inf" || lower == "infinity" {
    return Err(ParseErrorKind::NotFinite);
  }
  if let Some(payload) = lower
    .strip_prefix("snan")
    .or_else(|| lower.strip_prefix("nan"))
  {
    return Err(if payload.bytes().all(|b| b.is_ascii_digit()) {
      ParseErrorKind::NotFinite
    } else {
      ParseErrorKind::InvalidSyntax
    });
  }

  let (mantissa_text, exponent_text) = match body.split_once(['e', 'E']) {
    Some((mantissa_text, exponent_text)) => (mantissa_text, Some(exponent_text)),
    None => (body, None),
  };
  // `1` and `1.` mean the same, so a missing fraction and an empty one need not be told apart.
  let (integral_text, fraction_text) = mantissa_text.split_once('.').unwrap_or((mantissa_text, ""));
  // Empty, or a digit followed by digits and underscores.
  let is_run = |text: &str| {
    text.is_empty()
      || (text.starts_with(|c: char| c.is_ascii_digit())
        && text.chars().all(|c| c.is_ascii_digit() || c == '_'))
  };
  let no_digits = integral_text.is_empty() && fraction_text.is_empty();
  if !is_run(integral_text) || !is_run(fraction_text) || no_digits {
    return Err(ParseErrorKind::InvalidSyntax);
  }

  let exponent = match exponent_text {
    None => BigInt::ZERO,
    Some(text) => {
      let (exponent_negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
      };
      let valid = digits.chars().all(|c| c.is_ascii_digit() || c == '_')
        && digits.chars().any(|c| c.is_ascii_digit());
      if !valid {
        return Err(ParseErrorKind::InvalidSyntax);
      }
      let magnitude: BigInt = digits.replace('_', "").parse().expect("ASCII digits");
      if exponent_negative {
        -magnitude
      } else {
        magnitude
      }
    }
  };

  let mantissa: BigInt = format!("{integral_text}{fraction_text}")
    .replace('_', "")
    .parse()
    .expect("ASCII digits");
  if mantissa.sign() == Sign::NoSign {
    return Ok(BigInt::ZERO);
  }
  // A nonzero mantissa has at most `input.len()` digits, so scaled by an exponent beyond this
  // bound it is at least `10^100` or below `10^-100`: out of range, or inexact for every type.
  let bound = BigInt::from(input.len() + 100);
  if exponent > bound {
    return Err(ParseErrorKind::OutOfRange);
  }
  if exponent < -bound {
    return Err(ParseErrorKind::Inexact);
  }

  // The coefficient is `mantissa * 10^shift`, which must be an integer of at most
  // `integral + fraction` digits.
  let fraction_digits = fraction_text.replace('_', "").len();
  let shift =
    i64::try_from(exponent).expect("bounded") - fraction_digits as i64 + i64::from(fraction);
  let max = max_coeff(integral + fraction);
  let magnitude = if shift >= 0 {
    mantissa * pow10(shift as u32)
  } else {
    let divisor = pow10(shift.unsigned_abs() as u32);
    let quotient = &mantissa / &divisor;
    // The value's integral part has more than `integral` digits exactly when the truncated
    // coefficient exceeds the largest one, so this is checked before exactness.
    if quotient > max {
      return Err(ParseErrorKind::OutOfRange);
    }
    if mantissa % divisor != BigInt::ZERO {
      return Err(ParseErrorKind::Inexact);
    }
    quotient
  };
  if magnitude > max {
    return Err(ParseErrorKind::OutOfRange);
  }
  Ok(if negative { -magnitude } else { magnitude })
}

/// The result of converting `coeff` from `from` to `to` fraction digits, for a target with
/// `digits` digits in total and a signed or an unsigned storage.
fn oracle_convert(
  coeff: &BigInt,
  from: u32,
  to: u32,
  digits: u32,
  signed: bool,
) -> Result<BigInt, ConvertError> {
  // A negative value has no unsigned representation, however many digits are dropped.
  if !signed && coeff.sign() == Sign::Minus {
    return Err(ConvertError::OutOfRange);
  }
  let converted = if to >= from {
    coeff * pow10(to - from)
  } else {
    let divisor = pow10(from - to);
    let quotient = coeff / &divisor;
    if !in_range(&quotient, digits) {
      return Err(ConvertError::OutOfRange);
    }
    if coeff % &divisor != BigInt::ZERO {
      return Err(ConvertError::Inexact);
    }
    quotient
  };
  if in_range(&converted, digits) {
    Ok(converted)
  } else {
    Err(ConvertError::OutOfRange)
  }
}

/// Digit strings, possibly with leading zeros or extra fraction digits, for inputs that are
/// syntactically valid.
fn digit_text(max_len: usize) -> impl Strategy<Value = String> {
  proptest::collection::vec(
    prop_oneof![Just(b'0'), Just(b'9'), b'0'..=b'9'],
    1..=max_len,
  )
  .prop_map(|digits| String::from_utf8(digits).expect("ASCII digits"))
}

/// `digits` with up to two underscores after each digit, mostly none, as a digit run may have them
/// after its first digit.
fn separated(digits: String) -> impl Strategy<Value = String> {
  let underscores = prop_oneof![6 => Just(0usize), 1 => Just(1usize), 1 => Just(2usize)];
  proptest::collection::vec(underscores, digits.len()).prop_map(move |counts| {
    let mut text = String::new();
    for (digit, count) in digits.chars().zip(counts) {
      text.push(digit);
      text.extend(std::iter::repeat_n('_', count));
    }
    text
  })
}

/// The grammar's `digits`: a digit string from `digit_text` with underscores after its digits.
fn digit_run(max_len: usize) -> impl Strategy<Value = String> {
  digit_text(max_len).prop_flat_map(separated)
}

/// An exponent: `e` or `E`, an optional sign, then digits with underscores anywhere. Its magnitude
/// is mostly at most `max`, sometimes with leading zeros, and otherwise a run of up to 30 digits,
/// which can exceed every native integer.
fn exponent_text(max: u32) -> impl Strategy<Value = String> {
  let magnitude = prop_oneof![
    6 => (0..=max, 0usize..3)
      .prop_map(|(value, zeros)| format!("{}{value}", "0".repeat(zeros))),
    1 => digit_text(30),
  ];
  (
    prop_oneof![Just('e'), Just('E')],
    prop_oneof![Just(""), Just("+"), Just("-")],
    0usize..3,
    magnitude.prop_flat_map(separated),
  )
    .prop_map(|(marker, sign, leading, digits)| {
      format!("{marker}{sign}{}{digits}", "_".repeat(leading))
    })
}

/// Finite numbers in every syntactic form: any sign, a mantissa with or without either part and
/// the point, and an optional exponent.
fn number_text(integral: usize, fraction: usize, exponent: u32) -> impl Strategy<Value = String> {
  let mantissa = prop_oneof![
    digit_run(integral),
    digit_run(integral).prop_map(|digits| format!("{digits}.")),
    (digit_run(integral), digit_run(fraction))
      .prop_map(|(integral, fraction)| format!("{integral}.{fraction}")),
    digit_run(fraction).prop_map(|digits| format!(".{digits}")),
  ];
  (
    prop_oneof![Just(""), Just("+"), Just("-")],
    mantissa,
    proptest::option::of(exponent_text(exponent)),
  )
    .prop_map(|(sign, mantissa, exponent)| {
      format!("{sign}{mantissa}{}", exponent.unwrap_or_default())
    })
}

/// Infinities and NaNs, with any sign, random letter case and NaN payloads.
fn nonfinite_text() -> impl Strategy<Value = String> {
  let word = prop_oneof![
    Just(String::from("inf")),
    Just(String::from("infinity")),
    (
      prop_oneof![Just(""), Just("s")],
      proptest::option::of(digit_text(6))
    )
      .prop_map(|(signaling, payload)| format!("{signaling}nan{}", payload.unwrap_or_default())),
  ];
  (
    prop_oneof![Just(""), Just("+"), Just("-")],
    word,
    proptest::collection::vec(any::<bool>(), 10),
  )
    .prop_map(|(sign, word, upper)| {
      let mut text = String::from(sign);
      for (c, upper) in word
        .chars()
        .zip(upper.into_iter().chain(std::iter::repeat(false)))
      {
        text.push(if upper { c.to_ascii_uppercase() } else { c });
      }
      text
    })
}

/// Sequences of grammar pieces and near misses, which form valid and almost valid inputs far more
/// often than random characters do.
fn token_text() -> impl Strategy<Value = String> {
  let token = proptest::sample::select(vec![
    "0", "1", "9", "00", "_", "__", ".", "e", "E", "+", "-", "inf", "INF", "infinity", "Infinity",
    "nan", "NaN", "s", "i", "n", "f", " ", "\t", ",", "x", "\u{663}",
  ]);
  proptest::collection::vec(token, 0..10).prop_map(|tokens| tokens.concat())
}

/// Rewrites decimal text such as `-12.5` with its point moved `shift` places to the left and the
/// exponent `shift`, such as `-0.125e2` for 2 or `-1250e-2` for -2, keeping any sign.
fn with_exponent(text: &str, shift: i32, marker: char) -> String {
  let digits_start = text
    .find(|c: char| c.is_ascii_digit())
    .expect("decimal text has a digit");
  let (sign, magnitude) = text.split_at(digits_start);
  let (integral, fraction) = magnitude.split_once('.').unwrap_or((magnitude, ""));
  let mut digits = format!("{integral}{fraction}");
  // The point follows the first `point` digits.
  let mut point = integral.len() as i64 - i64::from(shift);
  if point < 0 {
    digits.insert_str(0, &"0".repeat(point.unsigned_abs() as usize));
    point = 0;
  }
  let point = point as usize;
  if point > digits.len() {
    digits.push_str(&"0".repeat(point - digits.len()));
  }
  let (integral, fraction) = digits.split_at(point);
  if fraction.is_empty() {
    format!("{sign}{integral}{marker}{shift}")
  } else {
    format!("{sign}{integral}.{fraction}{marker}{shift}")
  }
}

macro_rules! suite {
  (
    $name:ident: $ty:ty,
    $repr:ident,
    $sign:ident,
    integral = $integral:literal,
    fraction = $fraction:literal
  ) => {
    mod $name {
      use super::*;

      pub(super) type T = $ty;
      pub(super) const INTEGRAL: u32 = $integral;
      pub(super) const FRACTION: u32 = $fraction;
      pub(super) const DIGITS: u32 = $integral + $fraction;
      pub(super) const SIGNED: bool = suite!(@signed $sign);

      pub(super) fn big(value: T) -> BigInt {
        BigInt::from(value.repr())
      }

      fn from_big(coeff: &BigInt) -> Option<T> {
        $repr::try_from(coeff).ok().and_then(T::from_repr)
      }

      /// Coefficients at the edges of the range and at powers of ten and two inside it.
      fn edges() -> Vec<$repr> {
        let max = T::MAX.repr() as i128;
        let mut values = vec![0, 1, -1, max, -max, max - 1, -(max - 1)];
        for exp in 0..=DIGITS {
          let power = 10i128.pow(exp);
          values.extend([power, power - 1, -power, -(power - 1)]);
        }
        for exp in 0..127 {
          let power = 1i128 << exp;
          values.extend([power, power - 1, -power, -(power - 1)]);
        }
        values
          .into_iter()
          .filter(|&value| value.unsigned_abs() <= max as u128 && (SIGNED || value >= 0))
          .map(|value| value as $repr)
          .collect()
      }

      pub(super) fn coeff() -> BoxedStrategy<$repr> {
        let digits = suite!(@digits $sign, $repr);
        // Shifts keep random low bits, and a random sign for a signed storage, while bringing the
        // magnitude close to the decimal bound, which needs `bound_bits` bits.
        let bound_bits = max_coeff(DIGITS).bits() as u32;
        let value_bits = if SIGNED { $repr::BITS - 1 } else { $repr::BITS };
        let min_shift = value_bits.saturating_sub(bound_bits);
        let bits = (any::<$repr>(), min_shift..$repr::BITS).prop_filter_map(
          "outside the decimal range",
          |(bits, shift)| {
            let value = bits >> shift;
            T::from_repr(value).map(|_| value)
          },
        );
        let edges = proptest::sample::select(edges());
        prop_oneof![digits, bits, edges].boxed()
      }

      pub(super) fn value() -> BoxedStrategy<T> {
        coeff()
          .prop_map(|coeff| T::from_repr(coeff).expect("coefficient in range"))
          .boxed()
      }

      proptest! {
        #![proptest_config(config())]

        #[test]
        fn from_repr_checks_the_range(bits in any::<$repr>()) {
          let expected = in_range(&BigInt::from(bits), DIGITS);
          prop_assert_eq!(T::from_repr(bits).is_some(), expected);
          if let Some(value) = T::from_repr(bits) {
            prop_assert_eq!(value.repr(), bits);
          }
        }

        #[test]
        fn arithmetic_matches_big_integers(a in value(), b in value()) {
          let (big_a, big_b) = (big(a), big(b));
          prop_assert_eq!(a.checked_add(b).map(big), from_big(&(&big_a + &big_b)).map(big));
          prop_assert_eq!(a.checked_sub(b).map(big), from_big(&(&big_a - &big_b)).map(big));
          prop_assert_eq!(a.cmp(&b), big_a.cmp(&big_b));
          prop_assert_eq!(a == b, big_a == big_b);
          prop_assert_eq!(a.is_zero(), big_a.sign() == Sign::NoSign);
          prop_assert_eq!(a.is_positive(), big_a.sign() == Sign::Plus);
          suite!(@signed_arithmetic $sign, a, big_a);
        }

        #[test]
        fn display_matches_the_oracle(a in value(), precision in 0..(FRACTION as usize + 4)) {
          let text = oracle_text(&big(a), FRACTION, 0);
          prop_assert_eq!(a.to_string(), text.clone());
          prop_assert_eq!(format!("{a:?}"), text);
          prop_assert_eq!(
            format!("{a:.precision$}"),
            oracle_text(&big(a), FRACTION, precision)
          );
        }

        #[test]
        fn text_round_trips(
          a in value(),
          precision in 0..(FRACTION as usize + 4),
          width in 0usize..48,
        ) {
          prop_assert_eq!(T::parse(&a.to_string()), Ok(a));
          prop_assert_eq!(format!("{a:.precision$}").parse::<T>(), Ok(a));
          // Zero padding goes after the sign, so the padded text is still valid input.
          prop_assert_eq!(format!("{a:0width$.precision$}").parse::<T>(), Ok(a));
          // A `+` sign is accepted, for every storage.
          prop_assert_eq!(format!("{a:+}").parse::<T>(), Ok(a));
          prop_assert_eq!(format!("{a:+0width$.precision$}").parse::<T>(), Ok(a));
        }

        #[test]
        fn text_round_trips_with_exponents(
          a in value(),
          shift in -(DIGITS as i32 + 3)..=(DIGITS as i32 + 3),
          plus in any::<bool>(),
          upper in any::<bool>(),
        ) {
          let text = if plus { format!("{a:+}") } else { a.to_string() };
          let text = with_exponent(&text, shift, if upper { 'E' } else { 'e' });
          prop_assert_eq!(T::parse(&text), Ok(a), "text {:?}", text);
          prop_assert_eq!(
            oracle_parse(&text, INTEGRAL, FRACTION, SIGNED),
            Ok(big(a)),
            "text {:?}",
            text
          );
        }

        #[test]
        fn parse_matches_the_oracle_on_numbers(
          input in number_text(INTEGRAL as usize + 3, FRACTION as usize + 3, DIGITS + 3),
        ) {
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }

        #[test]
        fn parse_matches_the_oracle_on_nonfinite_text(input in nonfinite_text()) {
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }

        #[test]
        fn parse_matches_the_oracle_on_tokens(input in token_text()) {
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }

        #[test]
        fn parse_matches_the_oracle_on_digit_strings(
          negative in any::<bool>(),
          integral in digit_text(INTEGRAL as usize + 3),
          fraction in proptest::option::of(digit_text(FRACTION as usize + 3)),
        ) {
          let input = format!(
            "{}{integral}{}",
            if negative { "-" } else { "" },
            fraction.map(|digits| format!(".{digits}")).unwrap_or_default(),
          );
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }

        #[test]
        fn parse_matches_the_oracle_on_random_text(
          input in "[-+0-9.eE_iInNfFaAsStTyY,x \t\n\u{663}\u{ff11}]{0,16}",
        ) {
          let expected = oracle_parse(&input, INTEGRAL, FRACTION, SIGNED);
          let actual = T::parse(&input).map(big).map_err(|err| err.kind());
          prop_assert_eq!(actual, expected, "input {:?}", input);
        }
      }
    }
  };

  (@signed signed) => {
    true
  };
  (@signed unsigned) => {
    false
  };

  (@digits signed, $repr:ident) => {
    (any::<bool>(), digit_text(DIGITS as usize)).prop_map(|(negative, text)| {
      let magnitude: $repr = text.parse().expect("at most DIGITS digits fit");
      if negative { -magnitude } else { magnitude }
    })
  };
  (@digits unsigned, $repr:ident) => {
    digit_text(DIGITS as usize)
      .prop_map(|text| text.parse::<$repr>().expect("at most DIGITS digits fit"))
  };

  (@signed_arithmetic signed, $a:ident, $big_a:ident) => {
    prop_assert_eq!(big($a.neg()), -&$big_a);
    prop_assert_eq!(big(-$a), -&$big_a);
    prop_assert_eq!(big($a.abs()), BigInt::from($big_a.magnitude().clone()));
    prop_assert_eq!($a.is_negative(), $big_a.sign() == Sign::Minus);
  };
  (@signed_arithmetic unsigned, $a:ident, $big_a:ident) => {};
}

suite!(d1x1: Decimal<U1, U1>, i8, signed, integral = 1, fraction = 1);
suite!(d0x2: Decimal<U0, U2>, i8, signed, integral = 0, fraction = 2);
suite!(d2x0: Decimal<U2, U0>, i8, signed, integral = 2, fraction = 0);
suite!(d3x1: Decimal<U3, U1>, i16, signed, integral = 3, fraction = 1);
suite!(d4x5: Decimal<U4, U5>, i32, signed, integral = 4, fraction = 5);
suite!(d10x8: Decimal<U10, U8>, i64, signed, integral = 10, fraction = 8);
suite!(d12x8: Decimal<U12, U8>, i128, signed, integral = 12, fraction = 8);
suite!(d19x0: Decimal<U19, U0>, i128, signed, integral = 19, fraction = 0);
suite!(d19x19: Decimal<U19, U19>, i128, signed, integral = 19, fraction = 19);
suite!(d38x0: Decimal<U38, U0>, i128, signed, integral = 38, fraction = 0);
suite!(d0x38: Decimal<U0, U38>, i128, signed, integral = 0, fraction = 38);
suite!(wide: Decimal<U10, U8, i128>, i128, signed, integral = 10, fraction = 8);
suite!(d2x2_in_i64: Decimal<U2, U2, i64>, i64, signed, integral = 2, fraction = 2);
suite!(d0x36: Decimal<U0, U36>, i128, signed, integral = 0, fraction = 36);

suite!(u1x1: Decimal<U1, U1, u8>, u8, unsigned, integral = 1, fraction = 1);
suite!(u0x2: Decimal<U0, U2, u8>, u8, unsigned, integral = 0, fraction = 2);
suite!(u3x1: Decimal<U3, U1, u16>, u16, unsigned, integral = 3, fraction = 1);
suite!(u4x5: Decimal<U4, U5, u32>, u32, unsigned, integral = 4, fraction = 5);
suite!(u10x0: Decimal<U10, U0, u64>, u64, unsigned, integral = 10, fraction = 0);
suite!(u10x9: Decimal<U10, U9, u64>, u64, unsigned, integral = 10, fraction = 9);
suite!(u19x0: Decimal<U19, U0, u64>, u64, unsigned, integral = 19, fraction = 0);
suite!(u19x19: Decimal<U19, U19, u128>, u128, unsigned, integral = 19, fraction = 19);
suite!(u38x0: Decimal<U38, U0, u128>, u128, unsigned, integral = 38, fraction = 0);

/// The reference and the crate both agree with coefficients computed by hand, so that they cannot
/// share a mistake unnoticed.
#[test]
fn hand_computed_examples() {
  use ParseErrorKind::{Empty, Inexact, InvalidSyntax, NotFinite, OutOfRange};

  type Signed = Decimal<U4, U4>;
  type Unsigned = Decimal<U4, U4, u32>;

  // Coefficients for four integral and four fraction digits. An unsigned storage gives the same
  // result, except that it rejects every input that starts with `-`.
  let cases: &[(&str, Result<i64, ParseErrorKind>)] = &[
    ("", Err(Empty)),
    (" ", Err(InvalidSyntax)),
    ("\t", Err(InvalidSyntax)),
    ("1_000.000_1", Ok(10_000_001)),
    ("1__0", Ok(100_000)),
    ("1_", Ok(10_000)),
    ("5_.5", Ok(55_000)),
    ("1e_0_1", Ok(100_000)),
    ("1e_1_0", Err(OutOfRange)),
    ("+.5", Ok(5_000)),
    ("5.", Ok(50_000)),
    (".5e3", Ok(5_000_000)),
    ("1e+_3", Ok(10_000_000)),
    ("1e_3", Ok(10_000_000)),
    ("1e3_", Ok(10_000_000)),
    ("-1e-4", Ok(-1)),
    ("1E-04", Ok(1)),
    ("0e-5", Ok(0)),
    ("-0", Ok(0)),
    ("-0e5", Ok(0)),
    ("0e999999999", Ok(0)),
    ("0.0001e4", Ok(10_000)),
    ("1000e-3", Ok(10_000)),
    ("1e999999999", Err(OutOfRange)),
    ("1e-999999999", Err(Inexact)),
    ("12345", Err(OutOfRange)),
    ("0.00001", Err(Inexact)),
    ("12345.00001", Err(OutOfRange)),
    ("inf", Err(NotFinite)),
    ("-Infinity", Err(NotFinite)),
    ("+sNaN", Err(NotFinite)),
    ("NaN123", Err(NotFinite)),
    ("NaN1_2", Err(InvalidSyntax)),
    ("infinit", Err(InvalidSyntax)),
    ("_1", Err(InvalidSyntax)),
    ("._5", Err(InvalidSyntax)),
    ("1._5", Err(InvalidSyntax)),
    ("1e_", Err(InvalidSyntax)),
    ("1e_+5", Err(InvalidSyntax)),
    (".", Err(InvalidSyntax)),
    ("+", Err(InvalidSyntax)),
    ("-", Err(InvalidSyntax)),
    ("e5", Err(InvalidSyntax)),
    ("1e", Err(InvalidSyntax)),
    ("1e+", Err(InvalidSyntax)),
    ("1,5", Err(InvalidSyntax)),
    ("0x1", Err(InvalidSyntax)),
    ("1e999_x", Err(InvalidSyntax)),
  ];
  for &(input, expected) in cases {
    let unsigned = if input.starts_with('-') {
      Err(InvalidSyntax)
    } else {
      expected
    };
    let expected = expected.map(BigInt::from);
    let unsigned = unsigned.map(BigInt::from);
    assert_eq!(oracle_parse(input, 4, 4, true), expected, "input {input:?}");
    assert_eq!(
      oracle_parse(input, 4, 4, false),
      unsigned,
      "input {input:?}"
    );
    let signed_result = Signed::parse(input).map(|value| BigInt::from(value.repr()));
    assert_eq!(
      signed_result.map_err(|err| err.kind()),
      expected,
      "input {input:?}"
    );
    let unsigned_result = Unsigned::parse(input).map(|value| BigInt::from(value.repr()));
    assert_eq!(
      unsigned_result.map_err(|err| err.kind()),
      unsigned,
      "input {input:?}"
    );
  }
}

macro_rules! exact_mul {
  ($name:ident: $lhs:ident * $rhs:ident = $product:ty) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(a in $lhs::value(), b in $rhs::value()) {
        let product: $product = a.exact_mul(b);
        let expected = $lhs::big(a) * $rhs::big(b);
        prop_assert_eq!(BigInt::from(product.repr()), expected.clone());
        prop_assert_eq!(
          product.to_string(),
          oracle_text(&expected, $lhs::FRACTION + $rhs::FRACTION, 0)
        );
      }
    }
  };
}

exact_mul!(mul_i8_limit: d1x1 * d1x1 = Decimal<U2, U2>);
exact_mul!(mul_fraction_by_integral: d0x2 * d2x0 = Decimal<U2, U2>);
exact_mul!(mul_i16_by_i32: d3x1 * d4x5 = Decimal<U7, U6>);
exact_mul!(mul_price_by_quantity: d10x8 * d12x8 = Decimal<U22, U16>);
exact_mul!(mul_i128_limit: d19x0 * d19x0 = Decimal<U38, U0>);
exact_mul!(mul_fraction_limit: d0x2 * d0x36 = Decimal<U0, U38>);
exact_mul!(mul_explicit_storage: wide * d2x2_in_i64 = Decimal<U12, U10>);

// Two unsigned storages give the smallest unsigned storage for the product's digits.
exact_mul!(mul_u8_limit: u1x1 * u1x1 = Decimal<U2, U2, u16>);
exact_mul!(mul_u16_by_u32: u3x1 * u4x5 = Decimal<U7, U6, u64>);
exact_mul!(mul_u64_limit: u4x5 * u10x0 = Decimal<U14, U5, u64>);
exact_mul!(mul_u128_limit: u19x0 * u19x0 = Decimal<U38, U0, u128>);
exact_mul!(mul_unsigned_fraction: u10x9 * u0x2 = Decimal<U10, U11, u128>);
// A signed storage on either side gives the smallest signed storage.
exact_mul!(mul_signed_by_unsigned: d1x1 * u1x1 = Decimal<U2, U2, i16>);
exact_mul!(mul_unsigned_by_signed: u10x9 * d10x8 = Decimal<U20, U17, i128>);

macro_rules! convert {
  ($name:ident: $source:ident -> $target:ident) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(a in $source::value()) {
        let expected = oracle_convert(
          &$source::big(a),
          $source::FRACTION,
          $target::FRACTION,
          $target::DIGITS,
          $target::SIGNED,
        );
        let actual: Result<$target::T, ConvertError> = a.try_convert();
        prop_assert_eq!(actual.map($target::big), expected);
      }
    }
  };
}

convert!(convert_to_more_integral_digits: d10x8 -> d12x8);
convert!(convert_to_wider_storage: d10x8 -> wide);
convert!(convert_to_fewer_integral_digits: d12x8 -> d10x8);
convert!(convert_to_narrower_storage: wide -> d2x2_in_i64);
convert!(convert_i128_limit_to_i8_limit: d19x19 -> d1x1);
convert!(convert_i8_limit_to_i128_limit: d1x1 -> d19x19);
convert!(convert_to_fewer_fraction_digits: d0x38 -> d0x2);
convert!(convert_to_more_fraction_digits: d0x2 -> d0x38);
convert!(convert_integral_to_fraction: d38x0 -> d0x38);
convert!(convert_fraction_to_integral: d0x38 -> d38x0);
convert!(convert_mixed: d4x5 -> d3x1);

convert!(convert_signed_to_unsigned: d10x8 -> u10x9);
convert!(convert_signed_fraction_to_unsigned: d0x38 -> u1x1);
convert!(convert_signed_integral_to_unsigned: d38x0 -> u19x0);
convert!(convert_unsigned_to_signed: u19x0 -> d19x0);
convert!(convert_unsigned_to_narrower_signed: u10x9 -> d10x8);
convert!(convert_unsigned_to_fewer_digits: u19x19 -> u1x1);
convert!(convert_unsigned_to_more_digits: u1x1 -> u19x19);
convert!(convert_unsigned_integral_to_fraction: u38x0 -> u19x19);

macro_rules! widen {
  ($name:ident: $source:ident -> $target:ident) => {
    proptest! {
      #![proptest_config(config())]

      #[test]
      fn $name(a in $source::value()) {
        let widened: $target::T = a.widen();
        let expected = $source::big(a) * pow10($target::FRACTION - $source::FRACTION);
        prop_assert_eq!($target::big(widened), expected);
        prop_assert_eq!(widened.to_string(), a.to_string());
      }
    }
  };
}

widen!(widen_integral_digits: d10x8 -> d12x8);
widen!(widen_storage: d10x8 -> wide);
widen!(widen_i8_to_i128_limit: d1x1 -> d19x19);
widen!(widen_fraction_digits: d0x2 -> d0x38);
widen!(widen_integral_only: d2x0 -> d38x0);
widen!(widen_both: d2x2_in_i64 -> d4x5);

widen!(widen_u8_to_i8: u1x1 -> d1x1);
widen!(widen_u64_limit_to_i128: u19x0 -> d19x0);
widen!(widen_unsigned_to_signed: u10x9 -> d19x19);
widen!(widen_unsigned_storage: u10x9 -> u19x19);
widen!(widen_unsigned_fraction_digits: u0x2 -> u19x19);

/// Renders a random `f64`, including NaN, the infinities and subnormals, with one of the four
/// formatting traits the standard library implements for it.
fn f64_text() -> impl Strategy<Value = String> {
  (any::<f64>(), 0..4u8).prop_map(|(value, variant)| match variant {
    0 => format!("{value}"),
    1 => format!("{value:?}"),
    2 => format!("{value:e}"),
    _ => format!("{value:E}"),
  })
}

/// Whether `input`, after an optional sign, is `snan` followed by any number of ASCII digits or
/// `nan` followed by at least one, ignoring ASCII case: the General Decimal Arithmetic NaN forms
/// that `f64::from_str` has no syntax for (it accepts a bare `nan`, but neither `snan` nor a digit
/// payload).
fn gda_only_nan_form(input: &str) -> bool {
  let body = input
    .strip_prefix('-')
    .or_else(|| input.strip_prefix('+'))
    .unwrap_or(input);
  let lower = body.to_ascii_lowercase();
  match lower.strip_prefix("snan") {
    Some(payload) => payload.bytes().all(|b| b.is_ascii_digit()),
    None => lower
      .strip_prefix("nan")
      .is_some_and(|payload| !payload.is_empty() && payload.bytes().all(|b| b.is_ascii_digit())),
  }
}

/// `word`, with each letter's case chosen independently at random.
fn random_case(word: &'static str) -> impl Strategy<Value = String> {
  proptest::collection::vec(any::<bool>(), word.len()).prop_map(move |upper| {
    word
      .chars()
      .zip(upper)
      .map(|(c, upper)| if upper { c.to_ascii_uppercase() } else { c })
      .collect()
  })
}

/// One of `f64::from_str`'s number forms, with no `_`: 1-5 mantissa digits biased toward `0` and
/// `9`, as `D`, `D.`, `D.D` or `.D`, with an optional exponent of `e`/`E`, an optional sign, and
/// 1-3 digits.
fn f64_grammar_number() -> impl Strategy<Value = String> {
  let mantissa = prop_oneof![
    digit_text(5),
    digit_text(5).prop_map(|d| format!("{d}.")),
    (digit_text(5), digit_text(5)).prop_map(|(i, f)| format!("{i}.{f}")),
    digit_text(5).prop_map(|d| format!(".{d}")),
  ];
  let exponent = (
    prop_oneof![Just('e'), Just('E')],
    prop_oneof![Just(""), Just("+"), Just("-")],
    digit_text(3),
  )
    .prop_map(|(marker, sign, digits)| format!("{marker}{sign}{digits}"));
  (mantissa, proptest::option::of(exponent))
    .prop_map(|(mantissa, exponent)| format!("{mantissa}{}", exponent.unwrap_or_default()))
}

/// Strings built directly from the number grammar `f64::from_str`'s documentation describes: an
/// optional sign, then either `inf`, `infinity` or `nan` in random letter case, or a number. Never
/// contains `_`.
fn f64_grammar_text() -> impl Strategy<Value = String> {
  let word = prop_oneof![
    random_case("inf"),
    random_case("infinity"),
    random_case("nan")
  ];
  (
    prop_oneof![Just(""), Just("+"), Just("-")],
    prop_oneof![word, f64_grammar_number()],
  )
    .prop_map(|(sign, body)| format!("{sign}{body}"))
}

/// One byte a near miss may insert: digits, the point, an exponent marker, a sign, a space, or a
/// letter from one of the three words; never `_`, which `f64` rejects regardless of position.
fn near_miss_byte() -> impl Strategy<Value = u8> {
  proptest::sample::select(b"0123456789.eE+- naifs".to_vec())
}

/// An index into a string of length `len`: for deleting or duplicating the byte there (`len`
/// itself is out of bounds), or, with `inclusive`, for inserting before it (`len` means
/// appending). Weighted toward both ends, because an edit in the middle of a digit run almost
/// always changes only the value, while the start and the end are where this grammar's syntax
/// actually breaks.
fn edit_index(len: usize, inclusive: bool) -> impl Strategy<Value = usize> {
  let max = if inclusive { len } else { len - 1 };
  prop_oneof![2 => Just(0), 2 => Just(max), 1 => 0..=max]
}

/// An `f64_grammar_text()` string with exactly one byte deleted, inserted or duplicated. Many
/// results are invalid by one character (`1e`, `1..5`, `+`, `e5`, ...), which is what property (b)
/// below needs. `f64_grammar_text()` never produces an empty string, so every index is valid.
fn near_miss_text() -> impl Strategy<Value = String> {
  f64_grammar_text().prop_flat_map(|text| {
    let len = text.len();
    (
      edit_index(len, false),
      edit_index(len, true),
      0u8..3,
      near_miss_byte(),
    )
      .prop_map(move |(at, insert_at, kind, byte)| {
        let mut bytes = text.as_bytes().to_vec();
        match kind {
          0 => {
            bytes.remove(at);
          }
          1 => bytes.insert(insert_at, byte),
          _ => {
            let duplicated = bytes[at];
            bytes.insert(at, duplicated);
          }
        }
        String::from_utf8(bytes).expect("ASCII bytes")
      })
  })
}

proptest! {
  #![proptest_config(config())]

  /// `Decimal::parse` accepts every syntactic form that `f64::from_str` accepts; checked directly
  /// against the standard library, outside the per-type suites so this runs once, not once per
  /// suite.
  #[test]
  fn parse_accepts_every_f64_form(
    input in prop_oneof![
      token_text().prop_map(|text| text.replace('_', "")),
      f64_text(),
      f64_grammar_text(),
      near_miss_text(),
    ]
  ) {
    type Signed = Decimal<U2, U2>;
    type Unsigned = Decimal<U2, U2, u16>;

    let f64_ok = input.parse::<f64>().is_ok();

    let signed_result = Signed::parse(&input).map_err(|err| err.kind());
    let unsigned_result = Unsigned::parse(&input).map_err(|err| err.kind());

    if f64_ok {
      prop_assert!(
        !matches!(signed_result, Err(ParseErrorKind::InvalidSyntax)),
        "input {:?}",
        input
      );
      prop_assert_eq!(
        matches!(unsigned_result, Err(ParseErrorKind::InvalidSyntax)),
        input.starts_with('-'),
        "input {:?}",
        input
      );
    }

    // Beyond `_` separators, the crate's grammar is exactly `f64`'s forms plus the General Decimal
    // Arithmetic NaN payloads that `f64` lacks; a signed result beyond invalid syntax or an empty
    // input must fall into one of those two cases.
    let signed_recognized = !matches!(
      signed_result,
      Err(ParseErrorKind::InvalidSyntax) | Err(ParseErrorKind::Empty)
    );
    if signed_recognized {
      prop_assert!(f64_ok || gda_only_nan_form(&input), "input {:?}", input);
    }
  }
}
