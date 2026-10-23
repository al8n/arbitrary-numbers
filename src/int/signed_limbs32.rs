//! Upper layer for `[u32; N]`: two's complement integers of `N` limbs of `B = 32` bits, least
//! significant first.
//!
//! `limb_layer!` generates the layer, over `limbs32`.

use super::limb_layer::limb_layer;

limb_layer!(signed limb: u32, signed: i32, unsigned: limbs32);

#[cfg(test)]
mod tests;
