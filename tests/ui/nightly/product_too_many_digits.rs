use arbitrary_numbers::{
  Decimal,
  typenum::{U0, U19, U20},
};

fn main() {
  let lhs = Decimal::<U19, U0>::MAX;
  let rhs = Decimal::<U20, U0>::MAX;
  let _product = lhs.exact_mul(rhs);
}
