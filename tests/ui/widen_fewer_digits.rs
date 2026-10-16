use arbitrary_numbers::{
  Decimal,
  typenum::{U2, U4, U8, U10},
};

fn fewer_integral(value: Decimal<U10, U8>) -> Decimal<U2, U8> {
  value.widen()
}

fn fewer_fraction(value: Decimal<U10, U8>) -> Decimal<U10, U4> {
  value.widen()
}

fn main() {}
