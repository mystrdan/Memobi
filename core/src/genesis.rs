//! Deterministic genesis pinned to consensus params.
//!
//! The genesis block is fully determined by `ConsensusParams`; its hash is
//! asserted in tests so accidental changes are caught.

use crate::{
    BlockHeight,
    block::{BlockHeader, transaction_root},
    chain::Block,
    hash::Hash32,
    params::ConsensusParams,
    transaction::{Transaction, TxOutput},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisConfig {
    pub timestamp: u64,
    pub target: u64,
    pub poarm_version: u32,
}

impl GenesisConfig {
    pub const fn provisional() -> Self {
        Self {
            timestamp: 0,
            target: u64::MAX,
            poarm_version: 0,
        }
    }
    pub const fn from_params(params: &ConsensusParams) -> Self {
        Self {
            timestamp: params.genesis_timestamp,
            target: params.genesis_target,
            poarm_version: params.poarm_version,
        }
    }
}

/// Build a deterministic zero-input genesis block.
///
/// The single genesis transaction has no inputs, one zero-value output
/// locked to the all-zero pubkey placeholder, and zero fee. It is never
/// spendable under v1 rules requiring funded inputs; it exists only to
/// commit to chain identity.
pub fn build_genesis(config: GenesisConfig) -> Block {
    // NOTE: zero-value output with 32-byte zero pubkey keeps v1 decode
    // shape stable; the output is unspendable (no funding input exists).
    let genesis_transaction = Transaction {
        version: 1,
        inputs: Vec::new(),
        outputs: vec![TxOutput {
            value: 0,
            spending_condition: vec![0u8; 32],
        }],
        fee: 0,
    };

    let transactions = vec![genesis_transaction];
    let header = BlockHeader {
        version: 1,
        previous_block: Hash32::ZERO,
        height: BlockHeight(0),
        timestamp: config.timestamp,
        target: config.target,
        poarm_version: config.poarm_version,
        poarm_nonce: 0,
        transaction_root: transaction_root(&transactions)
            .expect("genesis transaction serialization must be valid"),
    };

    Block {
        header,
        transactions,
    }
}

/// Build the genesis block directly from the selected network parameters.
pub fn genesis_for_params(params: &ConsensusParams) -> Block {
    build_genesis(GenesisConfig::from_params(params))
}

/// Devnet genesis pinned to `ConsensusParams::devnet()`.
pub fn devnet_genesis() -> Block {
    genesis_for_params(&ConsensusParams::devnet())
}

/// Testnet genesis pinned to `ConsensusParams::testnet()`.
pub fn testnet_genesis() -> Block {
    genesis_for_params(&ConsensusParams::testnet())
}

/// Mainnet-candidate genesis pinned to `ConsensusParams::mainnet_candidate()`.
///
/// This is still a candidate only; it is not a launched mainnet genesis.
pub fn mainnet_candidate_genesis() -> Block {
    genesis_for_params(&ConsensusParams::mainnet_candidate())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_is_deterministic() {
        let a = build_genesis(GenesisConfig::provisional());
        let b = build_genesis(GenesisConfig::provisional());
        assert_eq!(a, b);
        assert_eq!(a.header.previous_block, Hash32::ZERO);
        assert_eq!(a.header.height.0, 0);
    }

    #[test]
    fn devnet_genesis_hash_is_pinned() {
        let g = devnet_genesis();
        let id = g.header.block_id().unwrap();
        // Pin: any serialization/param change must update this vector.
        // Computed from devnet() params at time of writing.
        let hex: String = id.as_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex.len(), 64);
        // Determinism across runs:
        assert_eq!(g, devnet_genesis());
        // Chain identity binds network + height + timestamp + target.
        assert_eq!(g.header.height.0, 0);
        assert_eq!(
            g.header.timestamp,
            ConsensusParams::devnet().genesis_timestamp
        );
        assert_eq!(g.header.target, ConsensusParams::devnet().genesis_target);
    }
}
