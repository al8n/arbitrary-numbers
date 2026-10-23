use numo::{
  Uint,
  typenum::{Shleft, U6, U288230376151711744},
};

// The bits of 2^58 limbs, 2^64, which `typenum` writes as zero in a `u32` and in a `u64`.
type Bits = Shleft<U288230376151711744, U6>;

fn main() {
  let _bits = Uint::<Bits, [u64; 288230376151711744]>::BITS;
  let _: Uint<Bits, [u64; 288230376151711744]> = Default::default();
}
