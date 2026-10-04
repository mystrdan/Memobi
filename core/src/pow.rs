//! Experimental proof-to-target helpers.
//!
//! The exact target representation is not consensus-frozen. This module keeps
//! the comparison isolated so the eventual PoARM target rule can replace it
//! without leaking into transaction or wallet code.

use crate::hash::Hash32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowTarget(pub u64);

/// Laboratory comparison using the first eight proof bytes as a big-endian
/// integer. This is deliberately versionless and must not be used as final
/// mainnet consensus until the target representation is frozen.
pub fn meets_target(proof: Hash32, target: PowTarget) -> bool {
    let value = u64::from_be_bytes(proof.as_bytes()[..8].try_into().unwrap());
    value <= target.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_at_target_is_valid() {
        let target = PowTarget(10);
        let proof = Hash32([0; 32]);
        assert!(meets_target(proof, target));
    }

    #[test]
    fn proof_above_target_is_invalid() {
        let target = PowTarget(10);
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(&11u64.to_be_bytes());
        assert!(!meets_target(Hash32(bytes), target));
    }
}
