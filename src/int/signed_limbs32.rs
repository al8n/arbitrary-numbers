//! Upper layer for `[u32; N]`: two's complement integers of `N` limbs of `B = 32` bits, least
//! significant first.
//!
//! Only the tests build it, over `limbs32`, to check the layer that `limb_layer!` generates for a
//! limb type other than `u64`.

use super::limb_layer::limb_layer;

limb_layer!(signed limb: u32, signed: i32, unsigned: limbs32);

#[cfg(test)]
mod tests;
