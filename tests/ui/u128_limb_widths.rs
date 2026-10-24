use numo::{
  Uint,
  typenum::{U200, U257},
};

fn main() {
  // Two 128-bit limbs hold 256 bits, which is less than 257.
  let _: Uint<U257, [u128; 2]> = Default::default();
  // 200 bits leave 56 bits of two 128-bit limbs unused, so not every bit pattern is a value.
  let _value = Uint::<U200, [u128; 2]>::from_limbs([0; 2]);
}
