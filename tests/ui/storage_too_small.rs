use numo::{
  Decimal,
  typenum::{U2, U8, U10},
};

fn main() {
  // A constant reports the unsatisfied bounds of its impl.
  let _zero = Decimal::<U10, U8, i32>::ZERO;
  // A conversion reports the storage diagnostic of its target.
  let _converted = Decimal::<U2, U2>::ZERO.try_convert::<U10, U8, i32>();
}
