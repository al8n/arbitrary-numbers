use numo::{Uint, typenum::U4294967296};

fn main() {
  // 2^29 bytes hold 2^32 bits, but the `u32` of `BITS` does not.
  let _bits = Uint::<U4294967296, [u8; 536870912]>::BITS;
  let _: Uint<U4294967296, [u8; 536870912]> = Default::default();
}
