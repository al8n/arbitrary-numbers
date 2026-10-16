// Each row of the digit table gets a test named after its `typenum` constant.
#![allow(non_snake_case)]

use core::any::TypeId;

use typenum::{U0, U1, U2, U3, U8, U10, U18, U19, U20, U38, Unsigned};

use super::{DigitCount, FitsDigits, Layout, SealedLayout};

/// Every storage with its largest value, smallest storage first.
const STORAGES: [(&str, u128); 5] = [
  ("i8", i8::MAX as u128),
  ("i16", i16::MAX as u128),
  ("i32", i32::MAX as u128),
  ("i64", i64::MAX as u128),
  ("i128", i128::MAX as u128),
];

fn assert_fits<Digits: FitsDigits<Repr>, Repr>() {}

macro_rules! test_digits {
  ($($digits:ident = $value:literal => [$min:ident $(, $wider:ident)*]),+ $(,)?) => {$(
    #[test]
    fn $digits() {
      type Digits = typenum::consts::$digits;
      assert_eq!(<Digits as Unsigned>::U32, $value);

      let max_coeff = 10u128.pow($value) - 1;
      let minimal = STORAGES
        .iter()
        .position(|&(name, _)| name == stringify!($min))
        .expect("the table names a known storage");
      assert!(max_coeff <= STORAGES[minimal].1, "the minimal storage holds the digits");
      if let Some(smaller) = minimal.checked_sub(1) {
        assert!(max_coeff > STORAGES[smaller].1, "the next smaller storage does not");
      }
      assert_eq!(
        TypeId::of::<<Digits as DigitCount>::Storage>(),
        TypeId::of::<$min>()
      );

      // `FitsDigits` is implemented for exactly the storages that hold the digits.
      assert_fits::<Digits, $min>();
      $(assert_fits::<Digits, $wider>();)*
      let listed = [stringify!($min) $(, stringify!($wider))*];
      let holding = STORAGES
        .iter()
        .filter(|&&(_, max)| max_coeff <= max)
        .map(|&(name, _)| name);
      assert!(listed.into_iter().eq(holding));
    }
  )+};
}

digit_table!(test_digits);

fn digits<I, F, R>() -> (u32, u32, u32)
where
  (I, F): Layout<R>,
{
  (
    <(I, F) as SealedLayout<R>>::INTEGRAL,
    <(I, F) as SealedLayout<R>>::FRACTION,
    <(I, F) as SealedLayout<R>>::DIGITS,
  )
}

#[test]
fn layout_digit_counts() {
  assert_eq!(digits::<U1, U1, i8>(), (1, 1, 2));
  assert_eq!(digits::<U0, U2, i8>(), (0, 2, 2));
  assert_eq!(digits::<U3, U0, i16>(), (3, 0, 3));
  assert_eq!(digits::<U10, U8, i64>(), (10, 8, 18));
  assert_eq!(digits::<U10, U8, i128>(), (10, 8, 18));
  assert_eq!(digits::<U18, U20, i128>(), (18, 20, 38));
  assert_eq!(digits::<U19, U19, i128>(), (19, 19, 38));
  assert_eq!(digits::<U0, U38, i128>(), (0, 38, 38));
  assert_eq!(digits::<U38, U0, i128>(), (38, 0, 38));
}
