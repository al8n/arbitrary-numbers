use core::cmp::Ordering;

use super::{RoundingMode, increments};

const MODES: [RoundingMode; 7] = [
  RoundingMode::Up,
  RoundingMode::Down,
  RoundingMode::Ceiling,
  RoundingMode::Floor,
  RoundingMode::HalfUp,
  RoundingMode::HalfDown,
  RoundingMode::HalfEven,
];

/// Rounds `quarters / 4` to an integer with `mode`, choosing between the integers below and above
/// it by their distances, independently of `increments`.
fn round_quarters(quarters: i64, mode: RoundingMode) -> i64 {
  let floor = quarters.div_euclid(4);
  if quarters.rem_euclid(4) == 0 {
    return floor;
  }
  let ceiling = floor + 1;
  let (toward_zero, away_from_zero) = if quarters < 0 {
    (ceiling, floor)
  } else {
    (floor, ceiling)
  };
  let below = quarters - 4 * floor;
  let above = 4 * ceiling - quarters;
  match mode {
    RoundingMode::Up => away_from_zero,
    RoundingMode::Down => toward_zero,
    RoundingMode::Ceiling => ceiling,
    RoundingMode::Floor => floor,
    RoundingMode::HalfUp | RoundingMode::HalfDown | RoundingMode::HalfEven => {
      match below.cmp(&above) {
        Ordering::Less => floor,
        Ordering::Greater => ceiling,
        Ordering::Equal => match mode {
          RoundingMode::HalfUp => away_from_zero,
          RoundingMode::HalfDown => toward_zero,
          _ if floor % 2 == 0 => floor,
          _ => ceiling,
        },
      }
    }
  }
}

/// Every combination of inputs, against values a quarter, a half and three quarters past a
/// quotient of each parity, of either sign.
#[test]
fn increments_matches_rounding_by_distance() {
  for mode in MODES {
    for negative in [false, true] {
      for quotient in [0i64, 1, 2, 3] {
        for (quarters, half) in [
          (1, Ordering::Less),
          (2, Ordering::Equal),
          (3, Ordering::Greater),
        ] {
          let magnitude = 4 * quotient + quarters;
          let value = if negative { -magnitude } else { magnitude };
          let away = round_quarters(value, mode).abs() > quotient;
          assert_eq!(
            increments(mode, negative, false, half, quotient % 2 == 1),
            away,
            "{mode:?}, value {value}/4"
          );
        }
        // An exact quotient never moves, whatever `half` says.
        for half in [Ordering::Less, Ordering::Equal, Ordering::Greater] {
          for odd in [false, true] {
            assert!(!increments(mode, negative, true, half, odd), "{mode:?}");
          }
        }
      }
    }
  }
}

/// The decision of each mode, written out case by case.
#[test]
fn increments_per_mode() {
  use Ordering::{Equal, Greater, Less};
  use RoundingMode::{Ceiling, Down, Floor, HalfDown, HalfEven, HalfUp, Up};

  for negative in [false, true] {
    for half in [Less, Equal, Greater] {
      for odd in [false, true] {
        assert!(increments(Up, negative, false, half, odd));
        assert!(!increments(Down, negative, false, half, odd));
        assert_eq!(increments(Ceiling, negative, false, half, odd), !negative);
        assert_eq!(increments(Floor, negative, false, half, odd), negative);
      }
      // The half modes ignore the sign; only a tie looks further.
      assert!(!increments(HalfUp, negative, false, Less, false));
      assert!(increments(HalfUp, negative, false, Equal, false));
      assert!(increments(HalfUp, negative, false, Greater, false));
      assert!(!increments(HalfDown, negative, false, Less, true));
      assert!(!increments(HalfDown, negative, false, Equal, true));
      assert!(increments(HalfDown, negative, false, Greater, true));
      assert!(!increments(HalfEven, negative, false, Less, true));
      assert!(!increments(HalfEven, negative, false, Equal, false));
      assert!(increments(HalfEven, negative, false, Equal, true));
      assert!(increments(HalfEven, negative, false, Greater, false));
    }
  }
}

#[test]
fn modes_are_plain_values() {
  fn assert_traits<T: Copy + Eq + core::hash::Hash + core::fmt::Debug + Send + Sync + 'static>() {}
  assert_traits::<RoundingMode>();
  assert_eq!(size_of::<RoundingMode>(), 1);
  assert_eq!(std::format!("{:?}", RoundingMode::HalfEven), "HalfEven");
}
