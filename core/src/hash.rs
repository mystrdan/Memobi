//! Fixed-size hash primitives used by the protocol.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Hash32(pub [u8; 32]);

impl Hash32 {
    pub const ZERO: Self = Self([0u8; 32]);

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Hash canonical protocol bytes with SHA-256.
///
/// The hash function is kept behind this small API so the consensus design can
/// evolve without leaking hashing details through every protocol type.
pub fn sha256(bytes: &[u8]) -> Hash32 {
    Hash32(Sha256::digest(bytes).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_deterministic() {
        assert_eq!(sha256(b"memobi"), sha256(b"memobi"));
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256(b""),
            Hash32([
                0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
                0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
                0x78, 0x52, 0xb8, 0x55,
            ])
        );
    }
}
