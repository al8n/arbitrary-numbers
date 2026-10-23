use numo::{Uint, typenum::U300};

fn main() {
  // The obligation of `Default` reports the diagnostic of the width check.
  let _: Uint<U300, [u64; 4]> = Default::default();
}
