use numo::{
  Decimal,
  typenum::{U0, U2, U155},
  u512,
};

fn main() {
  // 155 digits do not fit the widest storage: a constant reports the unsatisfied bounds of its impl.
  let _zero = Decimal::<U155, U0, u512>::ZERO;
  // A conversion reports the storage diagnostic of its target.
  let _converted = Decimal::<U2, U2>::ZERO.try_convert::<U155, U0, u512>();
}
