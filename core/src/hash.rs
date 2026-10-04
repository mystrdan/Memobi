//! Fixed-size hash primitives used by the protocol.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
