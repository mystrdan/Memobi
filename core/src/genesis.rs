//! Deterministic genesis construction for local/devnet experiments.
//!
//! Genesis economics and serialization are not mainnet-frozen.

use crate::{
    BlockHeight,
    block::{BlockHeader, transaction_root},
    chain::Block,
    hash::Hash32,
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
}

/// Build a deterministic zero-input genesis block.
///
/// The output is an ordinary transaction for now; final coinbase/genesis
/// semantics must be decided before consensus freeze.
pub fn build_genesis(config: GenesisConfig) -> Block {
    let genesis_transaction = Transaction {
        version: 1,
        inputs: Vec::new(),
        outputs: vec![TxOutput {
            value: 0,
            spending_condition: Vec::new(),
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
}
