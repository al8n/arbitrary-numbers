//! Property tests that compare `Decimal` with exact big-integer arithmetic.
//!
//! Coefficients come from three sources: random digit strings biased toward `0` and `9`, random
//! bit patterns scaled to the decimal bound, and the edges of the range. Parsing is checked against
//! an independent implementation of the grammar on random digit strings and on random text. Each
//! suite covers one decimal type, with a signed or an unsigned storage.

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
/// storage accepts a `-`.
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
    None => (false, input),
  };
  let (integral_text, fraction_text) = match body.split_once('.') {
    Some((integral_text, fraction_text)) => (integral_text, Some(fraction_text)),
    None => (body, None),
  };
  let is_digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
  if !is_digits(integral_text) || fraction_text.is_some_and(|text| !is_digits(text)) {
    return Err(ParseErrorKind::InvalidSyntax);
  }

  let significant = integral_text.trim_start_matches('0');
  if significant.len() > integral as usize {
    return Err(ParseErrorKind::OutOfRange);
  }
  let fraction_text = fraction_text.unwrap_or("");
  let (kept, dropped) = fraction_text.split_at(fraction_text.len().min(fraction as usize));
  if dropped.bytes().any(|b| b != b'0') {
    return Err(ParseErrorKind::Inexact);
  }

  let padding = "0".repeat(fraction as usize - kept.len());
  let digits = format!("{significant}{kept}{padding}");
  let magnitude = if digits.is_empty() {
    BigInt::ZERO
  } else {
    digits.parse::<BigInt>().expect("digits parse")
  };
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
          // The input grammar has no `+`.
          let with_sign = format!("{a:+}");
          if big(a).sign() == Sign::Minus {
            prop_assert_eq!(with_sign.parse::<T>(), Ok(a));
          } else {
            prop_assert_eq!(
              with_sign.parse::<T>().map_err(|err| err.kind()),
              Err(ParseErrorKind::InvalidSyntax)
            );
          }
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
          input in "[-+0-9.eE_ x\u{663}\u{ff11}]{0,12}",
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
