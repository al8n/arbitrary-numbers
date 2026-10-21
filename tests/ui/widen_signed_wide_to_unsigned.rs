use arbitrary_numbers::{
  Decimal, u192,
  typenum::{U18, U39},
};

fn signed_to_unsigned(value: Decimal<U39, U18>) -> Decimal<U39, U18, u192> {
  value.widen()
}

fn main() {}
