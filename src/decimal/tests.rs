use core::{
  fmt::{Debug, Display},
  hash::{BuildHasher, Hash},
  str::FromStr,
};
use std::{
  collections::hash_map::RandomState,
  format,
  string::{String, ToString},
  vec::Vec,
};

use typenum::{U0, U1, U2, U4, U5, U8, U10, U12, U15, U16, U19, U20, U22, U38};

use super::{ConvertError, Decimal, ParseError, ParseErrorKind};

/// `i8` at its 2-digit limit.
type D1x1 = Decimal<U1, U1>;
/// `i8` without integral digits.
type D0x2 = Decimal<U0, U2>;
/// `i8` without fraction digits.
type D2x0 = Decimal<U2, U0>;
type D2x2 = Decimal<U2, U2>;
type D5x4 = Decimal<U5, U4>;
type Price = Decimal<U10, U8>;
type Quantity = Decimal<U12, U8>;
/// `i128` at its 38-digit limit.
type D19x19 = Decimal<U19, U19>;
type D19x0 = Decimal<U19, U0>;
type D38x0 = Decimal<U38, U0>;
type D0x38 = Decimal<U0, U38>;
type WidePrice = Decimal<U10, U8, i128>;

const TEN_POW_19: i128 = 10_000_000_000_000_000_000;
const MAX_38: i128 = 99_999_999_999_999_999_999_999_999_999_999_999_999;

fn nines(count: usize) -> String {
  "9".repeat(count)
}

fn zeros(count: usize) -> String {
  "0".repeat(count)
}

#[test]
fn storage_layout() {
  let _: i8 = D1x1::ZERO.repr();
  let _: i8 = D0x2::ZERO.repr();
  let _: i8 = D2x0::ZERO.repr();
  let _: i16 = D2x2::ZERO.repr();
  let _: i32 = D5x4::ZERO.repr();
  let _: i64 = Price::ZERO.repr();
  let _: i128 = Quantity::ZERO.repr();
  let _: i128 = D19x19::ZERO.repr();
  let _: i128 = WidePrice::ZERO.repr();

  assert_eq!(size_of::<D1x1>(), size_of::<i8>());
  assert_eq!(size_of::<D2x2>(), size_of::<i16>());
  assert_eq!(size_of::<D5x4>(), size_of::<i32>());
  assert_eq!(size_of::<Price>(), size_of::<i64>());
  assert_eq!(align_of::<Price>(), align_of::<i64>());
  assert_eq!(size_of::<Quantity>(), size_of::<i128>());
  assert_eq!(align_of::<WidePrice>(), align_of::<i128>());
  assert_eq!(size_of::<Option<Price>>(), size_of::<Option<i64>>());
}

#[test]
fn trait_surface() {
  fn assert_traits<
    T: Copy + Eq + Ord + Hash + Default + Debug + Display + FromStr + Send + Sync + Unpin + 'static,
  >() {
  }

  assert_traits::<D1x1>();
  assert_traits::<D0x2>();
  assert_traits::<Price>();
  assert_traits::<D19x19>();
  assert_traits::<WidePrice>();
}

#[test]
fn constants() {
  assert_eq!(D1x1::ZERO.repr(), 0);
  assert_eq!(D1x1::MAX.repr(), 99);
  assert_eq!(D1x1::MIN.repr(), -99);
  assert_eq!(D1x1::ONE.repr(), 10);
  assert_eq!(D0x2::MAX.repr(), 99);
  assert_eq!(D0x2::MIN.repr(), -99);
  assert_eq!(D2x0::MAX.repr(), 99);
  assert_eq!(D2x0::ONE.repr(), 1);
  assert_eq!(D2x2::MAX.repr(), 9_999);
  assert_eq!(D2x2::ONE.repr(), 100);
  assert_eq!(D5x4::MAX.repr(), 999_999_999);
  assert_eq!(Price::MAX.repr(), 999_999_999_999_999_999);
  assert_eq!(Price::ONE.repr(), 100_000_000);
  assert_eq!(WidePrice::MAX.repr(), 999_999_999_999_999_999);
  assert_eq!(D19x19::MAX.repr(), MAX_38);
  assert_eq!(D19x19::MIN.repr(), -MAX_38);
  assert_eq!(D19x19::ONE.repr(), TEN_POW_19);
  assert_eq!(D38x0::MAX.repr(), MAX_38);
  assert_eq!(D38x0::ONE.repr(), 1);
  assert_eq!(D0x38::MAX.repr(), MAX_38);

  assert_eq!(D1x1::MIN, D1x1::MAX.neg());
  assert_eq!(D19x19::MIN, -D19x19::MAX);
  assert_eq!(Price::default(), Price::ZERO);
  assert_eq!(D0x2::default(), D0x2::ZERO);
}

#[test]
fn sign_predicates() {
  for (value, zero, negative, positive) in [
    (Price::ZERO, true, false, false),
    (Price::ONE, false, false, true),
    (Price::MAX, false, false, true),
    (Price::MIN, false, true, false),
    (Price::from_repr(-1).unwrap(), false, true, false),
  ] {
    assert_eq!(value.is_zero(), zero);
    assert_eq!(value.is_negative(), negative);
    assert_eq!(value.is_positive(), positive);
  }
}

#[test]
fn from_repr_checks_the_decimal_range() {
  assert_eq!(D1x1::from_repr(99), Some(D1x1::MAX));
  assert_eq!(D1x1::from_repr(-99), Some(D1x1::MIN));
  assert_eq!(D1x1::from_repr(100), None);
  assert_eq!(D1x1::from_repr(-100), None);
  assert_eq!(D1x1::from_repr(i8::MAX), None);
  assert_eq!(D1x1::from_repr(i8::MIN), None);

  assert_eq!(D19x19::from_repr(MAX_38), Some(D19x19::MAX));
  assert_eq!(D19x19::from_repr(-MAX_38), Some(D19x19::MIN));
  assert_eq!(D19x19::from_repr(MAX_38 + 1), None);
  assert_eq!(D19x19::from_repr(-MAX_38 - 1), None);
  assert_eq!(D19x19::from_repr(i128::MAX), None);
  assert_eq!(D19x19::from_repr(i128::MIN), None);

  assert_eq!(WidePrice::from_repr(1_000_000_000_000_000_000), None);
  assert_eq!(
    WidePrice::from_repr(999_999_999_999_999_999),
    Some(WidePrice::MAX)
  );
}

#[test]
fn checked_add_and_sub_at_the_i8_limit() {
  let ulp = D1x1::from_repr(1).unwrap();
  // The coefficient 100 fits in `i8`, so only the decimal bound rejects it.
  assert_eq!(D1x1::MAX.checked_add(ulp), None);
  assert_eq!(D1x1::MIN.checked_sub(ulp), None);
  assert_eq!(ulp.checked_sub(D1x1::MIN), None);
  // 99 + 99 overflows `i8` itself.
  assert_eq!(D1x1::MAX.checked_add(D1x1::MAX), None);
  assert_eq!(D1x1::MIN.checked_add(D1x1::MIN), None);
  assert_eq!(D1x1::MAX.checked_sub(D1x1::MIN), None);

  assert_eq!(D1x1::MAX.checked_add(D1x1::MIN), Some(D1x1::ZERO));
  assert_eq!(D1x1::MAX.checked_sub(D1x1::MAX), Some(D1x1::ZERO));
  assert_eq!(D1x1::ZERO.checked_sub(D1x1::MIN), Some(D1x1::MAX));
  assert_eq!(
    D1x1::MAX.checked_sub(ulp),
    Some(D1x1::from_repr(98).unwrap())
  );

  assert_eq!(D2x0::MAX.checked_add(D2x0::ONE), None);
  assert_eq!(D2x0::MIN.checked_sub(D2x0::ONE), None);
  assert_eq!(
    D0x2::MAX.checked_add(D0x2::from_repr(-1).unwrap()),
    D0x2::from_repr(98)
  );
}

#[test]
fn checked_add_and_sub_at_the_i128_limit() {
  let ulp = D19x19::from_repr(1).unwrap();
  // 10^38 fits in `i128`, so only the decimal bound rejects it.
  assert_eq!(D19x19::MAX.checked_add(ulp), None);
  assert_eq!(D19x19::MIN.checked_sub(ulp), None);
  // 2 * (10^38 - 1) overflows `i128` itself.
  assert_eq!(D19x19::MAX.checked_add(D19x19::MAX), None);
  assert_eq!(D19x19::MIN.checked_add(D19x19::MIN), None);
  assert_eq!(D19x19::MAX.checked_sub(D19x19::MIN), None);
  assert_eq!(D19x19::MIN.checked_sub(D19x19::MAX), None);

  assert_eq!(D19x19::MAX.checked_add(D19x19::MIN), Some(D19x19::ZERO));
  assert_eq!(D38x0::MAX.checked_add(D38x0::ONE), None);
  assert_eq!(
    D38x0::MAX.checked_sub(D38x0::ONE),
    D38x0::from_repr(MAX_38 - 1)
  );
}

#[test]
fn checked_add_and_sub_values() {
  let a: Price = "1.5".parse().unwrap();
  let b: Price = "-2.25".parse().unwrap();
  assert_eq!(a.checked_add(b), Some("-0.75".parse().unwrap()));
  assert_eq!(a.checked_sub(b), Some("3.75".parse().unwrap()));
  assert_eq!(b.checked_sub(a), Some("-3.75".parse().unwrap()));
  assert_eq!(a.checked_add(Price::ZERO), Some(a));
}

#[test]
fn neg_and_abs() {
  assert_eq!(D1x1::MIN.neg(), D1x1::MAX);
  assert_eq!(D1x1::MIN.abs(), D1x1::MAX);
  assert_eq!(D1x1::MAX.abs(), D1x1::MAX);
  assert_eq!(D19x19::MIN.abs(), D19x19::MAX);
  assert_eq!(Price::ZERO.neg(), Price::ZERO);
  assert!(!Price::ZERO.neg().is_negative());

  let value: Price = "-12.5".parse().unwrap();
  assert_eq!(-value, value.neg());
  assert_eq!(-(-value), value);
  assert_eq!(value.abs().to_string(), "12.5");
  assert_eq!((-value).to_string(), "12.5");
}

#[test]
fn ordering_and_equality() {
  let mut values: Vec<D2x2> = ["0.01", "-99.99", "0", "99.99", "-0.01", "1", "-1"]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();
  values.sort();
  let sorted: Vec<String> = values.iter().map(ToString::to_string).collect();
  assert_eq!(sorted, ["-99.99", "-1", "-0.01", "0", "0.01", "1", "99.99"]);
  assert!(D19x19::MIN < D19x19::ZERO && D19x19::ZERO < D19x19::MAX);
  assert_eq!(
    "1.50".parse::<D2x2>().unwrap(),
    "001.5".parse::<D2x2>().unwrap()
  );
  assert_ne!(D2x2::ONE, D2x2::ONE.neg());
}

#[test]
fn equal_values_hash_equally() {
  let state = RandomState::new();
  let a: Price = "1.50".parse().unwrap();
  let b: Price = "0001.5000".parse().unwrap();
  assert_eq!(a, b);
  assert_eq!(state.hash_one(a), state.hash_one(b));
}

fn parse_kind<T: FromStr<Err = ParseError>>(input: &str) -> Option<ParseErrorKind> {
  input.parse::<T>().err().map(|err| err.kind())
}

#[test]
fn parse_valid_inputs() {
  let cases: &[(&str, i16)] = &[
    ("0", 0),
    ("-0", 0),
    ("-0.00", 0),
    ("00", 0),
    ("0.0", 0),
    ("1", 100),
    ("-1", -100),
    ("12.34", 1_234),
    ("-12.34", -1_234),
    ("12.3", 1_230),
    ("012.30", 1_230),
    ("0.01", 1),
    ("-0.01", -1),
    ("99.99", 9_999),
    ("-99.99", -9_999),
    ("1.230000", 123),
    ("00000000000000000000000000000000000000000099.99", 9_999),
    ("1.50000000000000000000000000000000000000000000000000", 150),
  ];
  for &(input, coeff) in cases {
    assert_eq!(
      D2x2::parse(input).map(D2x2::repr),
      Ok(coeff),
      "input {input:?}"
    );
    assert_eq!(input.parse::<D2x2>().map(D2x2::repr), Ok(coeff));
  }
  assert!(!D2x2::parse("-0").unwrap().is_negative());
}

#[test]
fn parse_rejects_invalid_syntax() {
  for input in [
    "-",
    "+1",
    "+0",
    " 1",
    "1 ",
    "1\n",
    "1e5",
    "1E5",
    "1e-2",
    "1_0",
    ".5",
    "-.5",
    "1.",
    "-1.",
    ".",
    "--1",
    "-+1",
    "1-",
    "1.2.3",
    "1,5",
    "0x10",
    "NaN",
    "inf",
    "-inf",
    "x",
    "1.5x",
    "\u{0}",
    "\u{ff11}",
    "1\u{ff10}",
    "\u{663}",
  ] {
    assert_eq!(
      parse_kind::<D2x2>(input),
      Some(ParseErrorKind::InvalidSyntax),
      "input {input:?}"
    );
  }
  assert_eq!(parse_kind::<D2x2>(""), Some(ParseErrorKind::Empty));
}

#[test]
fn parse_range_and_exactness() {
  for (input, kind) in [
    ("100", ParseErrorKind::OutOfRange),
    ("-100", ParseErrorKind::OutOfRange),
    ("100.00", ParseErrorKind::OutOfRange),
    ("0100", ParseErrorKind::OutOfRange),
    ("1.234", ParseErrorKind::Inexact),
    ("-0.001", ParseErrorKind::Inexact),
    ("99.991", ParseErrorKind::Inexact),
    ("1.2300001", ParseErrorKind::Inexact),
  ] {
    assert_eq!(parse_kind::<D2x2>(input), Some(kind), "input {input:?}");
  }
}

#[test]
fn parse_error_priority() {
  // Syntax is checked over the whole input before range or exactness.
  assert_eq!(
    parse_kind::<D2x2>("99999x"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    parse_kind::<D2x2>("1.234x"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    parse_kind::<D2x2>("123.4x"),
    Some(ParseErrorKind::InvalidSyntax)
  );
  assert_eq!(
    parse_kind::<D2x2>("123.456."),
    Some(ParseErrorKind::InvalidSyntax)
  );
  // Out of range and inexact at once reports out of range.
  assert_eq!(
    parse_kind::<D2x2>("123.456"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D0x2>("1.001"),
    Some(ParseErrorKind::OutOfRange)
  );
}

#[test]
fn parse_at_the_i8_limit() {
  assert_eq!(D1x1::parse("9.9"), Ok(D1x1::MAX));
  assert_eq!(D1x1::parse("-9.9"), Ok(D1x1::MIN));
  assert_eq!(D1x1::parse("09.90").map(D1x1::repr), Ok(99));
  assert_eq!(parse_kind::<D1x1>("10"), Some(ParseErrorKind::OutOfRange));
  assert_eq!(parse_kind::<D1x1>("9.99"), Some(ParseErrorKind::Inexact));

  assert_eq!(D0x2::parse("0.99"), Ok(D0x2::MAX));
  assert_eq!(D0x2::parse("-0.5").map(D0x2::repr), Ok(-50));
  assert_eq!(D0x2::parse("00.010").map(D0x2::repr), Ok(1));
  assert_eq!(D0x2::parse("0"), Ok(D0x2::ZERO));
  assert_eq!(parse_kind::<D0x2>("1"), Some(ParseErrorKind::OutOfRange));
  assert_eq!(
    parse_kind::<D0x2>("-1.00"),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(parse_kind::<D0x2>("0.001"), Some(ParseErrorKind::Inexact));
  assert_eq!(
    parse_kind::<D0x2>(".99"),
    Some(ParseErrorKind::InvalidSyntax)
  );

  assert_eq!(D2x0::parse("99"), Ok(D2x0::MAX));
  assert_eq!(D2x0::parse("-99.000").map(D2x0::repr), Ok(-99));
  assert_eq!(parse_kind::<D2x0>("12.5"), Some(ParseErrorKind::Inexact));
  assert_eq!(parse_kind::<D2x0>("100"), Some(ParseErrorKind::OutOfRange));
}

#[test]
fn parse_at_the_i128_limit() {
  let max = format!("{}.{}", nines(19), nines(19));
  assert_eq!(D19x19::parse(&max), Ok(D19x19::MAX));
  assert_eq!(D19x19::parse(&format!("-{max}")), Ok(D19x19::MIN));
  assert_eq!(
    parse_kind::<D19x19>(&format!("1{}", zeros(19))),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D19x19>(&format!("{max}9")),
    Some(ParseErrorKind::Inexact)
  );

  assert_eq!(D38x0::parse(&nines(38)), Ok(D38x0::MAX));
  assert_eq!(D38x0::parse(&format!("-{}", nines(38))), Ok(D38x0::MIN));
  // 39 and more digits exceed `i128` as well; parsing must not overflow.
  assert_eq!(
    parse_kind::<D38x0>(&nines(39)),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<D38x0>(&format!("-{}", nines(60))),
    Some(ParseErrorKind::OutOfRange)
  );

  assert_eq!(D0x38::parse(&format!("0.{}", nines(38))), Ok(D0x38::MAX));
  assert_eq!(
    D0x38::parse(&format!("0.{}1", zeros(37))).map(D0x38::repr),
    Ok(1)
  );
  assert_eq!(
    parse_kind::<D0x38>(&format!("0.{}1", zeros(38))),
    Some(ParseErrorKind::Inexact)
  );
}

#[test]
fn parse_long_inputs() {
  let long_zeros = zeros(10_000);
  assert_eq!(
    Price::parse(&format!("{long_zeros}1")).map(Price::repr),
    Ok(100_000_000)
  );
  assert_eq!(
    Price::parse(&format!("1.{long_zeros}")).map(Price::repr),
    Ok(100_000_000)
  );
  assert_eq!(
    parse_kind::<Price>(&format!("1{long_zeros}")),
    Some(ParseErrorKind::OutOfRange)
  );
  assert_eq!(
    parse_kind::<Price>(&format!("1.{long_zeros}1")),
    Some(ParseErrorKind::Inexact)
  );
  assert_eq!(
    parse_kind::<Price>(&format!("1{long_zeros}x")),
    Some(ParseErrorKind::InvalidSyntax)
  );
}

#[test]
fn parse_in_const_context() {
  const TICK: Price = match Price::parse("0.01") {
    Ok(tick) => tick,
    Err(_) => panic!("invalid tick"),
  };
  const ERROR: Result<Price, ParseError> = Price::parse("1.000000001");

  assert_eq!(TICK.repr(), 1_000_000);
  assert_eq!(
    ERROR.map_err(|err| err.kind()),
    Err(ParseErrorKind::Inexact)
  );
}

#[test]
fn methods_in_const_context() {
  const TICK: Price = match Price::from_repr(1_000_000) {
    Some(tick) => tick,
    None => panic!("tick out of range"),
  };
  const SUM: Option<Price> = TICK.checked_add(TICK);
  const DIFFERENCE: Option<Price> = TICK.checked_sub(Price::ONE);
  const OVERFLOW: Option<Price> = Price::MAX.checked_add(TICK);
  const NEGATED: Price = TICK.neg();
  const ABSOLUTE: Price = NEGATED.abs();
  const SIGNS: [bool; 3] = [
    NEGATED.is_zero(),
    NEGATED.is_negative(),
    NEGATED.is_positive(),
  ];
  const COEFF: i64 = NEGATED.repr();

  assert_eq!(SUM.map(Price::repr), Some(2_000_000));
  assert_eq!(DIFFERENCE.map(Price::repr), Some(-99_000_000));
  assert_eq!(OVERFLOW, None);
  assert_eq!(ABSOLUTE, TICK);
  assert_eq!(SIGNS, [false, true, false]);
  assert_eq!(COEFF, -1_000_000);
}

#[test]
fn display_shortest_exact_form() {
  for (coeff, text) in [
    (0, "0"),
    (1, "0.01"),
    (10, "0.1"),
    (100, "1"),
    (1_230, "12.3"),
    (1_234, "12.34"),
    (-5, "-0.05"),
    (-100, "-1"),
    (9_999, "99.99"),
    (-9_999, "-99.99"),
  ] {
    let value = D2x2::from_repr(coeff).unwrap();
    assert_eq!(value.to_string(), text);
    assert_eq!(format!("{value:?}"), text);
  }

  assert_eq!(D2x0::from_repr(-7).unwrap().to_string(), "-7");
  assert_eq!(D2x0::MAX.to_string(), "99");
  assert_eq!(D0x2::from_repr(5).unwrap().to_string(), "0.05");
  assert_eq!(D0x2::MIN.to_string(), "-0.99");
  assert_eq!(D1x1::MIN.to_string(), "-9.9");

  let max = format!("{}.{}", nines(19), nines(19));
  assert_eq!(D19x19::MAX.to_string(), max);
  assert_eq!(D19x19::MIN.to_string(), format!("-{max}"));
  assert_eq!(D19x19::ONE.to_string(), "1");
  assert_eq!(D38x0::MIN.to_string(), format!("-{}", nines(38)));
  assert_eq!(
    D0x38::from_repr(1).unwrap().to_string(),
    format!("0.{}1", zeros(37))
  );
  assert_eq!(D0x38::MAX.to_string(), format!("0.{}", nines(38)));

  // The text depends only on the value, not on the storage.
  let narrow: Price = "-1234.5678".parse().unwrap();
  let wide: WidePrice = narrow.widen();
  assert_eq!(narrow.to_string(), wide.to_string());
}

#[test]
fn display_precision_pads_but_never_rounds() {
  let value = D2x2::from_repr(1_230).unwrap();
  assert_eq!(format!("{value:.0}"), "12.3");
  assert_eq!(format!("{value:.1}"), "12.3");
  assert_eq!(format!("{value:.2}"), "12.30");
  assert_eq!(format!("{value:.5}"), "12.30000");

  let digits = D2x2::from_repr(125).unwrap();
  assert_eq!(format!("{digits:.1}"), "1.25");
  assert_eq!(format!("{digits:.0}"), "1.25");

  assert_eq!(format!("{:.2}", D2x2::ZERO), "0.00");
  assert_eq!(format!("{:.0}", D2x2::ZERO), "0");
  assert_eq!(format!("{:.3}", D2x2::ONE), "1.000");
  assert_eq!(format!("{:.0}", D2x2::ONE), "1");
  assert_eq!(format!("{:.3}", D2x2::from_repr(-5).unwrap()), "-0.050");
  assert_eq!(format!("{:.4}", D2x0::MAX), "99.0000");
  assert_eq!(format!("{:.2?}", D2x2::ONE), "1.00");

  let tiny = D2x2::from_repr(1).unwrap();
  assert_eq!(format!("{tiny:.200}"), format!("0.01{}", zeros(198)));
  assert_eq!(
    format!("{:.100}", D19x19::MAX),
    format!("{}.{}{}", nines(19), nines(19), zeros(81))
  );
}

#[test]
fn display_flags_match_integers() {
  for value in [-99i8, -42, -7, 0, 7, 42, 99] {
    let decimal = D2x0::from_repr(value).unwrap();
    macro_rules! assert_same {
      ($($spec:tt),+ $(,)?) => {$(
        assert_eq!(
          format!($spec, decimal),
          format!($spec, value),
          "format {} of {}",
          $spec,
          value
        );
      )+};
    }
    assert_same!(
      "{}",
      "{:1}",
      "{:5}",
      "{:<5}",
      "{:^5}",
      "{:>5}",
      "{:^6}",
      "{:*<6}",
      "{:*^6}",
      "{:*>6}",
      "{:\u{e9}^7}",
      "{:+}",
      "{:+6}",
      "{:<+6}",
      "{:05}",
      "{:+05}",
      "{:<05}",
      "{:^05}",
      "{:x>+05}",
      "{:02}",
      "{:?}",
      "{:5?}",
      "{:<5?}",
      "{:^+7?}",
      "{:05?}",
    );
  }
}

#[test]
fn display_flags_with_fraction_digits() {
  let value = D2x2::from_repr(-1_230).unwrap();
  assert_eq!(format!("{value:8}"), "   -12.3");
  assert_eq!(format!("{value:>8}"), "   -12.3");
  assert_eq!(format!("{value:<8}"), "-12.3   ");
  assert_eq!(format!("{value:^8}"), " -12.3  ");
  assert_eq!(format!("{value:*^9}"), "**-12.3**");
  assert_eq!(format!("{value:2}"), "-12.3");
  assert_eq!(format!("{value:08}"), "-00012.3");
  assert_eq!(format!("{value:<08}"), "-00012.3");
  assert_eq!(format!("{value:08.2}"), "-0012.30");
  assert_eq!(format!("{value:>10.3}"), "   -12.300");
  assert_eq!(format!("{value:^12?}"), "   -12.3    ");

  let positive = value.abs();
  assert_eq!(format!("{positive:+}"), "+12.3");
  assert_eq!(format!("{positive:+08}"), "+00012.3");
  assert_eq!(format!("{positive:08}"), "000012.3");
  assert_eq!(format!("{positive:+.3}"), "+12.300");
  assert_eq!(format!("{:+}", D2x2::ZERO), "+0");
  assert_eq!(format!("{:+.1}", D2x2::ZERO), "+0.0");

  for spec_width in [70, 200] {
    let padded = format!("{positive:0spec_width$}");
    assert_eq!(padded.len(), spec_width);
    assert_eq!(padded, format!("{}12.3", zeros(spec_width - 4)));
    let filled = format!("{positive:_>spec_width$}");
    assert_eq!(filled, format!("{}12.3", "_".repeat(spec_width - 4)));
  }
}

#[test]
fn exact_mul() {
  let price: Price = "64250.5".parse().unwrap();
  let quantity: Quantity = "-0.0015".parse().unwrap();
  let notional: Decimal<U22, U16> = price.exact_mul(quantity);
  assert_eq!(notional.repr(), -963_757_500_000_000_000);
  assert_eq!(notional.to_string(), "-96.37575");

  // `MAX * MAX` at the 38-digit limit.
  let square: D38x0 = D19x0::MAX.exact_mul(D19x0::MAX);
  assert_eq!(square.repr(), (TEN_POW_19 - 1) * (TEN_POW_19 - 1));
  let negative: D38x0 = D19x0::MIN.exact_mul(D19x0::MAX);
  assert_eq!(negative, square.neg());
  let price_square: Decimal<U20, U16> = Price::MAX.exact_mul(Price::MAX);
  assert_eq!(price_square.repr(), 999_999_999_999_999_999i128.pow(2));

  // `i8` operands produce an `i16` result.
  let small: Decimal<U2, U2> = D1x1::MAX.exact_mul(D1x1::MIN);
  assert_eq!(small.to_string(), "-98.01");
  let fraction_only: Decimal<U0, U4> = D0x2::MAX.exact_mul(D0x2::MAX);
  assert_eq!(fraction_only.to_string(), "0.9801");
  let integral_only: Decimal<U2, U2> = D0x2::MAX.exact_mul(D2x0::MAX);
  assert_eq!(integral_only.to_string(), "98.01");

  // Mixed storages.
  let mixed: Decimal<U15, U12> = D5x4::MAX.exact_mul(WidePrice::MIN);
  assert_eq!(mixed.repr(), -999_999_999i128 * 999_999_999_999_999_999i128);
  assert_eq!(D5x4::ZERO.exact_mul(WidePrice::MIN).repr(), 0);
}

#[test]
fn widen() {
  let small: D2x2 = "-12.5".parse().unwrap();
  let price: Price = small.widen();
  assert_eq!(price.repr(), -1_250_000_000);
  let wide: WidePrice = small.widen();
  assert_eq!(wide.repr(), -1_250_000_000);

  // Same digits in a smaller storage.
  let stored: Decimal<U2, U2, i64> = small.widen();
  let back: D2x2 = stored.widen();
  assert_eq!(back, small);

  let max: D19x19 = D1x1::MAX.widen();
  assert_eq!(max.repr(), 99 * TEN_POW_19 / 10);
  let min: Decimal<U19, U19> = D1x1::MIN.widen();
  assert_eq!(min, max.neg());
  let integral: D38x0 = D2x0::MIN.widen();
  assert_eq!(integral.repr(), -99);
  let fraction: D0x38 = D0x2::MAX.widen();
  assert_eq!(fraction.to_string(), "0.99");
  let same: D19x19 = D19x19::MIN.widen();
  assert_eq!(same, D19x19::MIN);
}

#[test]
fn try_convert() {
  let value: D2x2 = "-1.20".parse().unwrap();
  assert_eq!(value.try_convert::<U2, U2, i16>(), Ok(value));
  assert_eq!(
    value.try_convert::<U2, U1, i16>().map(|v| v.repr()),
    Ok(-12)
  );
  assert_eq!(
    value.try_convert::<U10, U8, i64>(),
    Ok(value.widen::<U10, U8, i64>())
  );

  // A dropped digit is not zero.
  let inexact: D2x2 = "-1.25".parse().unwrap();
  assert_eq!(
    inexact.try_convert::<U2, U1, i16>(),
    Err(ConvertError::Inexact)
  );
  assert_eq!(
    inexact.try_convert::<U2, U0, i8>(),
    Err(ConvertError::Inexact)
  );

  // Range, at the boundary.
  let below: D2x2 = "9.99".parse().unwrap();
  assert_eq!(
    below.try_convert::<U1, U2, i16>().map(|v| v.repr()),
    Ok(999)
  );
  let limit: D2x2 = "10".parse().unwrap();
  assert_eq!(
    limit.try_convert::<U1, U2, i16>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    limit.neg().try_convert::<U1, U2, i16>(),
    Err(ConvertError::OutOfRange)
  );

  // Out of range and inexact at once reports out of range.
  assert_eq!(
    D2x2::MAX.try_convert::<U1, U1, i8>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D2x2::MIN.try_convert::<U1, U1, i8>(),
    Err(ConvertError::OutOfRange)
  );

  // `Integral = 0` targets.
  let half: D2x2 = "-0.5".parse().unwrap();
  assert_eq!(half.try_convert::<U0, U2, i8>().map(|v| v.repr()), Ok(-50));
  assert_eq!(
    D2x2::ONE.try_convert::<U0, U2, i8>(),
    Err(ConvertError::OutOfRange)
  );

  // Scaling up overflows `i128` before the range check.
  assert_eq!(
    D38x0::MAX.try_convert::<U0, U38, i128>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(
    D38x0::ONE.try_convert::<U0, U38, i128>(),
    Err(ConvertError::OutOfRange)
  );
  assert_eq!(D38x0::ZERO.try_convert::<U0, U38, i128>(), Ok(D0x38::ZERO));

  // Scaling down by 38 digits.
  assert_eq!(
    D0x38::MAX.try_convert::<U38, U0, i128>(),
    Err(ConvertError::Inexact)
  );
  let tenth: D0x38 = "-0.1".parse().unwrap();
  assert_eq!(tenth.try_convert::<U1, U1, i8>().map(|v| v.repr()), Ok(-1));

  // Narrower storage and fewer digits, when the value fits.
  let price: Price = "-12.5".parse().unwrap();
  assert_eq!(
    price.try_convert::<U2, U2, i16>().map(|v| v.repr()),
    Ok(-1_250)
  );
  assert_eq!(
    price.try_convert::<U2, U0, i8>(),
    Err(ConvertError::Inexact)
  );
  let whole: Price = "12.00000000".parse().unwrap();
  assert_eq!(whole.try_convert::<U2, U0, i8>().map(|v| v.repr()), Ok(12));
  assert_eq!(
    Quantity::MAX.try_convert::<U10, U8, i64>(),
    Err(ConvertError::OutOfRange)
  );
  let quantity: Quantity = "1234567890.12345678".parse().unwrap();
  assert_eq!(
    quantity.try_convert::<U10, U8, i64>().map(|v| v.repr()),
    Ok(123_456_789_012_345_678)
  );
}

#[test]
fn errors() {
  let empty = D2x2::parse("").unwrap_err();
  assert_eq!(empty.kind(), ParseErrorKind::Empty);
  assert_eq!(empty.to_string(), "cannot parse decimal from empty string");
  assert_eq!(
    D2x2::parse("x").unwrap_err().to_string(),
    "invalid decimal syntax"
  );
  assert_eq!(
    D2x2::parse("100").unwrap_err().to_string(),
    "decimal out of range of the target type"
  );
  assert_eq!(
    D2x2::parse("0.001").unwrap_err().to_string(),
    "decimal has more fraction digits than the target type holds"
  );
  assert_eq!(
    ConvertError::OutOfRange.to_string(),
    "decimal out of range of the target type"
  );
  assert_eq!(
    ConvertError::Inexact.to_string(),
    "decimal cannot be represented exactly in the target type"
  );

  let errors: [&dyn core::error::Error; 2] = [&empty, &ConvertError::Inexact];
  assert!(errors.iter().all(|err| err.source().is_none()));
}
