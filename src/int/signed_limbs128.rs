//! Upper layer for `[u128; N]`: two's complement integers of `N` limbs of `B = 128` bits, least
//! significant first.
//!
//! `limb_layer!` generates the layer, over `limbs128`.

use super::limb_layer::limb_layer;

limb_layer!(signed limb: u128, signed: i128, unsigned: limbs128);

#[cfg(test)]
mod tests;
