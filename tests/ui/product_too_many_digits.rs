use arbitrary_numbers::{
  Decimal,
  typenum::{U0, U48, U49},
};

fn main() {
  let lhs = Decimal::<U48, U0>::MAX;
  let rhs = Decimal::<U49, U0>::MAX;
  let _product = lhs.exact_mul(rhs);
}
