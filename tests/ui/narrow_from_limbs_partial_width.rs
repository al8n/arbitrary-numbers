use numo::{
  Int, Uint,
  typenum::{U7, U9},
};

fn main() {
  // 9 bits leave 7 bits of two bytes unused, so not every bit pattern is a value.
  let _value = Uint::<U9, [u8; 2]>::from_limbs([0; 2]);
  let _value = Int::<U7, [u16; 1]>::from_limbs([0]);
}
