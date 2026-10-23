//! Aliases of [`Uint`] and [`Int`] for common widths.
//!
//! An alias is named after its sign and width, as the primitive integers are: `uN` is
//! `Uint<UN, [u64; N / 64]>` and `iN` is `Int<UN, [u64; N / 64]>`, an integer of `N` bits that
//! fills `N / 64` limbs of 64 bits. The aliases cover 192, 256, 320, 384, 448 and 512 bits, the
//! wide storages of [`Decimal`](crate::Decimal), and 768, 1024, 2048 and 4096 bits. A native width,
//! such as 64 or 128 bits, has no alias, which would shadow the primitive type of the same name.
//! Any other width is written as the full type, such as `Uint<U200, [u64; 4]>`, and so is every
//! storage in narrower limbs, such as `Uint<U256, [u8; 32]>`: the other limb types have no alias.
//!
//! The crate root re-exports every alias, so `numo::u256` and `numo::aliases::u256` are the same
//! type.

// The aliases follow the names of the primitive integers.
#![allow(non_camel_case_types)]

use typenum::{U192, U256, U320, U384, U448, U512, U768, U1024, U2048, U4096};

use crate::{Int, Uint};

/// A 192-bit unsigned integer, stored in three 64-bit limbs.
pub type u192 = Uint<U192, [u64; 3]>;

/// A 256-bit unsigned integer, stored in four 64-bit limbs.
pub type u256 = Uint<U256, [u64; 4]>;

/// A 320-bit unsigned integer, stored in five 64-bit limbs.
pub type u320 = Uint<U320, [u64; 5]>;

/// A 384-bit unsigned integer, stored in six 64-bit limbs.
pub type u384 = Uint<U384, [u64; 6]>;

/// A 448-bit unsigned integer, stored in seven 64-bit limbs.
pub type u448 = Uint<U448, [u64; 7]>;

/// A 512-bit unsigned integer, stored in eight 64-bit limbs.
pub type u512 = Uint<U512, [u64; 8]>;

/// A 768-bit unsigned integer, stored in twelve 64-bit limbs.
pub type u768 = Uint<U768, [u64; 12]>;

/// A 1024-bit unsigned integer, stored in sixteen 64-bit limbs.
pub type u1024 = Uint<U1024, [u64; 16]>;

/// A 2048-bit unsigned integer, stored in 32 64-bit limbs.
pub type u2048 = Uint<U2048, [u64; 32]>;

/// A 4096-bit unsigned integer, stored in 64 64-bit limbs.
pub type u4096 = Uint<U4096, [u64; 64]>;

/// A 192-bit signed integer, stored in three 64-bit limbs in two's complement.
pub type i192 = Int<U192, [u64; 3]>;

/// A 256-bit signed integer, stored in four 64-bit limbs in two's complement.
pub type i256 = Int<U256, [u64; 4]>;

/// A 320-bit signed integer, stored in five 64-bit limbs in two's complement.
pub type i320 = Int<U320, [u64; 5]>;

/// A 384-bit signed integer, stored in six 64-bit limbs in two's complement.
pub type i384 = Int<U384, [u64; 6]>;

/// A 448-bit signed integer, stored in seven 64-bit limbs in two's complement.
pub type i448 = Int<U448, [u64; 7]>;

/// A 512-bit signed integer, stored in eight 64-bit limbs in two's complement.
pub type i512 = Int<U512, [u64; 8]>;

/// A 768-bit signed integer, stored in twelve 64-bit limbs in two's complement.
pub type i768 = Int<U768, [u64; 12]>;

/// A 1024-bit signed integer, stored in sixteen 64-bit limbs in two's complement.
pub type i1024 = Int<U1024, [u64; 16]>;

/// A 2048-bit signed integer, stored in 32 64-bit limbs in two's complement.
pub type i2048 = Int<U2048, [u64; 32]>;

/// A 4096-bit signed integer, stored in 64 64-bit limbs in two's complement.
pub type i4096 = Int<U4096, [u64; 64]>;
