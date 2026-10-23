use numo::{Uint, typenum::U0};

fn main() {
  // `U0` is not `NonZero`, so a constant reports the unsatisfied bounds of its impl.
  let _zero = Uint::<U0, [u64; 1]>::ZERO;
}
