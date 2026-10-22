use arbitrary_numbers::{
  Decimal, i192,
  typenum::{U0, U2, U58},
};

fn main() {
  // 58 digits need an `i256`: a constant reports the unsatisfied bounds of its impl.
  let _zero = Decimal::<U58, U0, i192>::ZERO;
  // A conversion reports the storage diagnostic of its target.
  let _converted = Decimal::<U2, U2>::ZERO.try_convert::<U58, U0, i192>();
}
