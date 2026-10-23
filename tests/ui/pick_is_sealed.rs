use numo::{Pick, typenum::B0};

// The seal of `Pick` holds only for the digit counts of the crate, so no other type can be picked
// for, even a local one.
struct Local;

impl Pick<Local> for B0 {
  type Storage = i8;
}

fn main() {}
