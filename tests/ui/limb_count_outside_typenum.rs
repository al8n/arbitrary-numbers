use numo::{Uint, typenum::U8};

fn main() {
  // `typenum` has no number for 1025, so an array of 1025 limbs holds no width: the type exists,
  // but it has no constants or constructors.
  let _max = Uint::<U8, [u8; 1025]>::MAX;
}
