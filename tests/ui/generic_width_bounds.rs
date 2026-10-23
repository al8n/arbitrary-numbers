use numo::{FitsBits, Uint, typenum::Unsigned};

// The bounds of a generic caller do not prove that the width fills the limbs.
fn from_limbs<B: FitsBits<[u64; 4]>>(limbs: [u64; 4]) -> Uint<B, [u64; 4]> {
  Uint::<B, [u64; 4]>::from_limbs(limbs)
}

fn largest<B: FitsBits<[u64; 2]>>() -> Uint<B, [u64; 2]> {
  Uint::<B, [u64; 2]>::MAX
}

// Nor that the limbs hold the width.
fn caller<B: Unsigned>() {
  let _largest = largest::<B>();
}

fn main() {}
