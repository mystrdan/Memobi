//! Native block primitives.
//!
//! The exact consensus fields remain under active design; this module gives
//! the project a deterministic skeleton without freezing PoARM or economics.

use crate::{codec::{put_u32_le, put_u64_le, Encode}, hash::sha256, Hash32, BlockHeight, ProtocolError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockHeader {
    pub version: u32,
    pub previous_block: Hash32,
    pub height: BlockHeight,
    pub timestamp: u64,
    pub target: u64,
    pub poarm_version: u32,
    pub poarm_nonce: u64,
    pub transaction_root: Hash32,
}

impl Encode for BlockHeader {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtocolError> {
        put_u32_le(out, self.version);
        out.extend_from_slice(self.previous_block.as_bytes());
        put_u64_le(out, self.height.0);
        put_u64_le(out, self.timestamp);
        put_u64_le(out, self.target);
        put_u32_le(out, self.poarm_version);
        put_u64_le(out, self.poarm_nonce);
        out.extend_from_slice(self.transaction_root.as_bytes());
        Ok(())
    }
}

impl BlockHeader {
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut out = Vec::with_capacity(4 + 32 + 8 + 8 + 8 + 4 + 8 + 32);
        self.encode(&mut out)?;
        Ok(out)
    }

    /// Compute the block-header identifier from its canonical serialization.
    pub fn block_id(&self) -> Result<Hash32, ProtocolError> {
        Ok(sha256(&self.encode_to_vec()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_encoding_is_fixed_and_deterministic() {
        let header = BlockHeader {
            version: 1,
            previous_block: Hash32([1u8; 32]),
            height: BlockHeight(42),
            timestamp: 100,
            target: u64::MAX,
            poarm_version: 0,
            poarm_nonce: 9,
            transaction_root: Hash32([2u8; 32]),
        };
        let encoded = header.encode_to_vec().unwrap();
        assert_eq!(encoded.len(), 104);
        assert_eq!(encoded, header.encode_to_vec().unwrap());
    }
}
