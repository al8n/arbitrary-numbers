//! Helpers for the unit tests that compare with `num-bigint`: conversions between limb arrays and
//! big integers, the limits and powers of ten that they test, rounding by distance, and fewer
//! samples and exponents under Miri.

use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use std::vec::Vec;

use crate::RoundingMode;

/// Every rounding mode.
pub(crate) const MODES: [RoundingMode; 7] = [
  RoundingMode::Up,
  RoundingMode::Down,
  RoundingMode::Ceiling,
  RoundingMode::Floor,
  RoundingMode::HalfUp,
  RoundingMode::HalfDown,
  RoundingMode::HalfEven,
];

/// `value / divisor` rounded to an integer with `mode`, for `divisor > 0`.
///
/// Picks between the integers below and above the exact quotient by their distances to it, and
/// breaks a tie by direction or parity. The crate instead compares the remainder of a division
/// toward zero with its distance to the divisor.
pub(crate) fn round(value: &BigInt, divisor: &BigInt, mode: RoundingMode) -> BigInt {
  // `floor * divisor <= value < (floor + 1) * divisor`; `/` rounds toward zero.
  let mut floor = value / divisor;
  if &floor * divisor > *value {
    floor -= 1u8;
  }
  let below = value - &floor * divisor;
  if below.sign() == Sign::NoSign {
    return floor;
  }
  let ceiling = &floor + 1u8;
  let above = &ceiling * divisor - value;
  let negative = value.sign() == Sign::Minus;
  let (toward_zero, away_from_zero) = if negative {
    (ceiling.clone(), floor.clone())
  } else {
    (floor.clone(), ceiling.clone())
  };
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
          _ if (&floor % 2u8).sign() == Sign::NoSign => floor,
          _ => ceiling,
        },
      }
    }
  }
}

/// The integer that `limbs` holds, least significant limb first, in two's complement if `signed`.
pub(crate) fn from_limbs(limbs: &[u64], signed: bool) -> BigInt {
  let bytes: Vec<u8> = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
  if signed {
    BigInt::from_signed_bytes_le(&bytes)
  } else {
    BigInt::from_bytes_le(Sign::Plus, &bytes)
  }
}

/// The smallest integer of `bits` bits, signed or unsigned.
pub(crate) fn min(bits: u32, signed: bool) -> BigInt {
  if signed {
    -(BigInt::from(1u8) << (bits - 1))
  } else {
    BigInt::ZERO
  }
}

/// The largest integer of `bits` bits, signed or unsigned.
pub(crate) fn max(bits: u32, signed: bool) -> BigInt {
  (BigInt::from(1u8) << (bits - u32::from(signed))) - 1u8
}

/// `value` in `N` limbs, least significant first, in two's complement if `signed`, or `None` if it
/// is out of range.
pub(crate) fn to_limbs<const N: usize>(value: &BigInt, signed: bool) -> Option<[u64; N]> {
  let negative = value.sign() == Sign::Minus;
  // The shortest two's complement or unsigned encoding, which fits exactly when the value does.
  let mut bytes = match (signed, negative) {
    (true, _) => value.to_signed_bytes_le(),
    (false, false) => value.to_bytes_le().1,
    (false, true) => return None,
  };
  if bytes.len() > 8 * N {
    return None;
  }
  bytes.resize(8 * N, if negative { 0xff } else { 0 });
  let mut limbs = [0; N];
  for (limb, chunk) in limbs.iter_mut().zip(bytes.as_chunks::<8>().0) {
    *limb = u64::from_le_bytes(*chunk);
  }
  Some(limbs)
}

/// `10^exp`.
pub(crate) fn pow10(exp: u32) -> BigInt {
  BigInt::from(10u8).pow(exp)
}

/// The largest `exp` such that `10^exp <= max`.
pub(crate) fn max_exp(max: &BigInt) -> u32 {
  let ten = BigInt::from(10u8);
  let mut exp = 0;
  // `10^(exp + 1)`, multiplied up by one factor of ten per step.
  let mut next = ten.clone();
  while next <= *max {
    exp += 1;
    next *= &ten;
  }
  exp
}

/// The exponents `0..=max_exp` to test, or under Miri only a few of them.
///
/// Scaling and division in `int::limbs` step by at most 19 digits (`MAX_STEP`), so the exponents
/// kept are the edges of one and two steps, plus the top. Under Miri big-integer arithmetic is slow,
/// and on a 32-bit target every exponent would use up its address space.
pub(crate) fn exponents(max_exp: u32) -> Vec<u32> {
  if !cfg!(miri) {
    return (0..=max_exp).collect();
  }
  let mut exps: Vec<u32> = [0, 1, 18, 19, 20, 38, 39, max_exp.saturating_sub(1), max_exp]
    .into_iter()
    .filter(|&exp| exp <= max_exp)
    .collect();
  exps.sort_unstable();
  exps.dedup();
  exps
}

/// `samples`, or every third of them under Miri, which runs big-integer arithmetic slowly.
pub(crate) fn thin<T>(samples: Vec<T>) -> Vec<T> {
  let step = if cfg!(miri) { 3 } else { 1 };
  samples.into_iter().step_by(step).collect()
}
