use numo::{
  Int, Uint,
  typenum::{U9, U17, U65},
};

fn main() {
  // A constant reports the unsatisfied bounds of its impl for every limb type.
  let _max = Uint::<U9, [u8; 1]>::MAX;
  let _min = Int::<U17, [u16; 1]>::MIN;
  let _zero = Uint::<U65, [u32; 2]>::ZERO;
}
