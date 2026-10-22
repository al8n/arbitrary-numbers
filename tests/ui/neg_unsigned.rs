use arbitrary_numbers::{Decimal, typenum::U2};

fn main() {
  let value = Decimal::<U2, U2, u16>::ONE;
  let _negated = -value;
}
