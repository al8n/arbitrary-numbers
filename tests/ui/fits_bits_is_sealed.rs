use numo::{FitsBits, typenum::U8};

// The seal of `FitsBits` holds for every storage, even a local one.
struct Local;

impl FitsBits<Local> for U8 {}

fn main() {}
