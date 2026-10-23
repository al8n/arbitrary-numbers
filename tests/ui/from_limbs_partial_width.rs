use numo::{
  Int, Uint,
  typenum::{U7, U200},
};

fn main() {
  // 200 bits leave 56 bits of four limbs unused, so not every bit pattern is a value.
  let _value = Uint::<U200, [u64; 4]>::from_limbs([0; 4]);
  let _value = Int::<U7, [u64; 1]>::from_limbs([0]);
}
