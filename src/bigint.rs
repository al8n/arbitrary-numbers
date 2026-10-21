//! Helpers for the unit tests that compare with `num-bigint`: conversions between limb arrays and
//! big integers, the limits and powers of ten that they test, and fewer samples under Miri.

use num_bigint::{BigInt, Sign};
use std::vec::Vec;

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
  let mut exp = 0;
  while pow10(exp + 1) <= *max {
    exp += 1;
  }
  exp
}

/// `samples`, or every third of them under Miri, which runs big-integer arithmetic slowly.
pub(crate) fn thin<T>(samples: Vec<T>) -> Vec<T> {
  let step = if cfg!(miri) { 3 } else { 1 };
  samples.into_iter().step_by(step).collect()
}
