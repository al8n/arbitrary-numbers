use numo::{Uint, typenum::U17};

fn main() {
  // The obligation of `Default` reports the diagnostic of the width check.
  let _: Uint<U17, [u8; 2]> = Default::default();
}
