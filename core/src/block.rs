//! Native block primitives.
//!
//! The exact consensus fields remain under active design; this module gives
//! the project a deterministic skeleton without freezing PoARM or economics.

use crate::{
    BlockHeight, ProtocolError,
    codec::{Encode, put_u32_le, put_u64_le},
    hash::{Hash32, sha256},
    transaction::Transaction,
};

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

/// Compute a deterministic binary Merkle-style root for transaction IDs.
///
/// This helper is suitable for devnet experiments. The exact tree construction
/// remains subject to consensus review before mainnet serialization is frozen.
pub fn transaction_root(transactions: &[Transaction]) -> Result<Hash32, ProtocolError> {
    if transactions.is_empty() {
        return Ok(Hash32::ZERO);
    }

    let mut level = transactions
        .iter()
        .map(Transaction::txid)
        .collect::<Result<Vec<_>, _>>()?;

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let right = pair.get(1).copied().unwrap_or(pair[0]);
            let mut bytes = Vec::with_capacity(64);
            bytes.extend_from_slice(pair[0].as_bytes());
            bytes.extend_from_slice(right.as_bytes());
            next.push(sha256(&bytes));
        }
        level = next;
    }

    Ok(level[0])
}

impl BlockHeader {
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut out = Vec::with_capacity(4 + 32 + 8 + 8 + 8 + 4 + 8 + 32);
        self.encode(&mut out)?;
        Ok(out)
    }

    /// Build the canonical preimage seed for the experimental PoARM workload.
    ///
    /// The nonce is excluded so miners can vary it without changing the seed.
    /// Epoch is explicit so future workload parameters can depend on chain state.
    pub fn poarm_seed(&self, epoch: u64) -> Result<Hash32, ProtocolError> {
        self.poarm_seed_with_domain("MEMOBI-POARM-V0", epoch)
    }

    /// Build the PoARM seed using the consensus-selected domain.
    ///
    /// The domain is consensus data, so production callers should use this
    /// parameterized form rather than relying on the provisional compatibility
    /// wrapper above.
    pub fn poarm_seed_with_params(
        &self,
        params: &crate::params::ConsensusParams,
        epoch: u64,
    ) -> Result<Hash32, ProtocolError> {
        self.poarm_seed_with_domain(params.poarm_domain, epoch)
    }

    fn poarm_seed_with_domain(&self, domain: &str, epoch: u64) -> Result<Hash32, ProtocolError> {
        let mut bytes = Vec::with_capacity(domain.len() + 4 + 32 + 8 + 8 + 8 + 4 + 32 + 8);
        bytes.extend_from_slice(domain.as_bytes());
        put_u32_le(&mut bytes, self.version);
        bytes.extend_from_slice(self.previous_block.as_bytes());
        put_u64_le(&mut bytes, self.height.0);
        put_u64_le(&mut bytes, self.timestamp);
        put_u64_le(&mut bytes, self.target);
        put_u32_le(&mut bytes, self.poarm_version);
        bytes.extend_from_slice(self.transaction_root.as_bytes());
        put_u64_le(&mut bytes, epoch);
        Ok(sha256(&bytes))
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
    fn transaction_root_is_deterministic() {
        let tx = Transaction {
            version: 1,
            inputs: vec![crate::TxInput {
                previous_output: crate::OutPoint {
                    txid: Hash32([7u8; 32]),
                    index: 0,
                },
                unlocking_data: vec![0u8; 96],
            }],
            outputs: vec![crate::TxOutput {
                value: 10,
                spending_condition: vec![0u8; 32],
            }],
            fee: 0,
        };
        let root = transaction_root(std::slice::from_ref(&tx)).unwrap();
        assert_eq!(root, tx.txid().unwrap());
        assert_eq!(root, transaction_root(&[tx]).unwrap());
    }

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
        assert_eq!(header.block_id().unwrap(), sha256(&encoded));
    }

    #[test]
    fn poarm_seed_uses_consensus_domain() {
        let header = BlockHeader {
            version: 1,
            previous_block: Hash32([1u8; 32]),
            height: BlockHeight(4),
            timestamp: 100,
            target: u64::MAX,
            poarm_version: 0,
            poarm_nonce: 0,
            transaction_root: Hash32([2u8; 32]),
        };
        let params = crate::params::ConsensusParams::devnet();
        assert_eq!(
            header.poarm_seed(7).unwrap(),
            header.poarm_seed_with_params(&params, 7).unwrap()
        );
        let mut alternate = params;
        alternate.poarm_domain = "MEMOBI-POARM-TEST";
        assert_ne!(
            header.poarm_seed_with_params(&params, 7).unwrap(),
            header.poarm_seed_with_params(&alternate, 7).unwrap()
        );
    }

    #[test]
    fn poarm_seed_excludes_nonce_but_changes_with_epoch() {
        let mut a = BlockHeader {
            version: 1,
            previous_block: Hash32([1u8; 32]),
            height: BlockHeight(4),
            timestamp: 100,
            target: u64::MAX,
            poarm_version: 0,
            poarm_nonce: 1,
            transaction_root: Hash32([2u8; 32]),
        };
        let first = a.poarm_seed(7).unwrap();
        a.poarm_nonce = 99;
        assert_eq!(first, a.poarm_seed(7).unwrap());
        assert_ne!(first, a.poarm_seed(8).unwrap());
    }
}
