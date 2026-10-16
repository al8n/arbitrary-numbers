// Each row of the digit table gets a test named after its `typenum` constant.
#![allow(non_snake_case)]

use core::any::TypeId;

use typenum::{B0, B1, U0, U1, U2, U3, U8, U9, U10, U18, U19, U20, U38, Unsigned};

use super::{DigitCount, FitsDigits, Layout, Pick, SealedLayout, SealedStorage, WidenRepr};

/// Every signed storage with its largest value, smallest storage first.
const SIGNED: [(&str, u128); 5] = [
  ("i8", i8::MAX as u128),
  ("i16", i16::MAX as u128),
  ("i32", i32::MAX as u128),
  ("i64", i64::MAX as u128),
  ("i128", i128::MAX as u128),
];

/// Every unsigned storage with its largest value, smallest storage first.
const UNSIGNED: [(&str, u128); 5] = [
  ("u8", u8::MAX as u128),
  ("u16", u16::MAX as u128),
  ("u32", u32::MAX as u128),
  ("u64", u64::MAX as u128),
  ("u128", u128::MAX),
];

fn assert_fits<Digits: FitsDigits<Repr>, Repr>() {}

/// Checks the storages that a row lists for one sign class: the first holds `max_coeff` while the
/// next smaller storage of the class does not, and the list is every storage of the class that
/// holds it.
fn assert_class(storages: &[(&str, u128)], max_coeff: u128, listed: &[&str]) {
  let minimal = storages
    .iter()
    .position(|&(name, _)| name == listed[0])
    .expect("the table names a known storage");
  assert!(
    max_coeff <= storages[minimal].1,
    "the minimal storage holds the digits"
  );
  if let Some(smaller) = minimal.checked_sub(1) {
    assert!(
      max_coeff > storages[smaller].1,
      "the next smaller storage does not"
    );
  }
  let holding = storages
    .iter()
    .filter(|&&(_, max)| max_coeff <= max)
    .map(|&(name, _)| name);
  assert!(listed.iter().copied().eq(holding));
}

macro_rules! test_digits {
  ($(
    $digits:ident = $value:literal =>
      [$signed:ident $(, $wider_signed:ident)*],
      [$unsigned:ident $(, $wider_unsigned:ident)*]
  ),+ $(,)?) => {$(
    #[test]
    fn $digits() {
      type Digits = typenum::consts::$digits;
      assert_eq!(<Digits as Unsigned>::U32, $value);

      let max_coeff = 10u128.pow($value) - 1;
      assert_class(
        &SIGNED,
        max_coeff,
        &[stringify!($signed) $(, stringify!($wider_signed))*],
      );
      assert_class(
        &UNSIGNED,
        max_coeff,
        &[stringify!($unsigned) $(, stringify!($wider_unsigned))*],
      );
      assert_eq!(
        TypeId::of::<<Digits as DigitCount>::Signed>(),
        TypeId::of::<$signed>()
      );
      assert_eq!(
        TypeId::of::<<Digits as DigitCount>::Unsigned>(),
        TypeId::of::<$unsigned>()
      );

      // `FitsDigits` is implemented for every listed storage.
      assert_fits::<Digits, $signed>();
      $(assert_fits::<Digits, $wider_signed>();)*
      assert_fits::<Digits, $unsigned>();
      $(assert_fits::<Digits, $wider_unsigned>();)*
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

  assert_eq!(digits::<U1, U1, u8>(), (1, 1, 2));
  assert_eq!(digits::<U10, U9, u64>(), (10, 9, 19));
  assert_eq!(digits::<U19, U0, u64>(), (19, 0, 19));
  assert_eq!(digits::<U10, U9, u128>(), (10, 9, 19));
  assert_eq!(digits::<U38, U0, u128>(), (38, 0, 38));
}

#[test]
fn storage_signs() {
  fn signed<R: SealedStorage>() -> bool {
    R::SIGNED
  }

  assert!(signed::<i8>() && signed::<i16>() && signed::<i32>());
  assert!(signed::<i64>() && signed::<i128>());
  assert!(!signed::<u8>() && !signed::<u16>() && !signed::<u32>());
  assert!(!signed::<u64>() && !signed::<u128>());
  assert_eq!(
    TypeId::of::<<i64 as SealedStorage>::IsUnsigned>(),
    TypeId::of::<B0>()
  );
  assert_eq!(
    TypeId::of::<<u64 as SealedStorage>::IsUnsigned>(),
    TypeId::of::<B1>()
  );
}

#[test]
fn pick_by_sign() {
  fn picked<B: Pick<D>, D>() -> TypeId
  where
    B::Storage: 'static,
  {
    TypeId::of::<B::Storage>()
  }

  assert_eq!(picked::<B0, U2>(), TypeId::of::<i8>());
  assert_eq!(picked::<B1, U2>(), TypeId::of::<u8>());
  // 19 digits need an `i128` but fit a `u64`.
  assert_eq!(picked::<B0, U19>(), TypeId::of::<i128>());
  assert_eq!(picked::<B1, U19>(), TypeId::of::<u64>());
  assert_eq!(picked::<B0, U38>(), TypeId::of::<i128>());
  assert_eq!(picked::<B1, U38>(), TypeId::of::<u128>());
}

#[test]
fn widen_repr_pairs() {
  fn assert_widens<Source: WidenRepr<Target>, Target>() {}

  macro_rules! assert_pairs {
    ($($source:ident => [$($target:ident),+]),+ $(,)?) => {
      $($(assert_widens::<$source, $target>();)+)+
    };
  }

  // Every unsigned storage converts to every storage, and every signed one to every signed one;
  // the remaining pairs fail to compile.
  assert_pairs!(
    u8 => [u8, u16, u32, u64, u128, i8, i16, i32, i64, i128],
    u16 => [u8, u16, u32, u64, u128, i8, i16, i32, i64, i128],
    u32 => [u8, u16, u32, u64, u128, i8, i16, i32, i64, i128],
    u64 => [u8, u16, u32, u64, u128, i8, i16, i32, i64, i128],
    u128 => [u8, u16, u32, u64, u128, i8, i16, i32, i64, i128],
    i8 => [i8, i16, i32, i64, i128],
    i16 => [i8, i16, i32, i64, i128],
    i32 => [i8, i16, i32, i64, i128],
    i64 => [i8, i16, i32, i64, i128],
    i128 => [i8, i16, i32, i64, i128],
  );
}
