use numo::{
  Decimal,
  typenum::{U2, U4},
};

fn signed_to_unsigned(value: Decimal<U2, U2>) -> Decimal<U2, U4, u32> {
  value.widen()
}

fn main() {}
