use numo::{Uint, typenum::U4294967296};

fn main() {
  // 2^26 limbs hold 2^32 bits, but the `u32` of `BITS` does not.
  let _bits = Uint::<U4294967296, [u64; 67108864]>::BITS;
  let _: Uint<U4294967296, [u64; 67108864]> = Default::default();
}
