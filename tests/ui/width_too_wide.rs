use numo::{
  Int, Uint,
  typenum::{U65, U300},
};

fn main() {
  // A constant reports the unsatisfied bounds of its impl.
  let _max = Uint::<U300, [u64; 4]>::MAX;
  let _min = Int::<U65, [u64; 1]>::MIN;
}
