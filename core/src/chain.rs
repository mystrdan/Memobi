//! Minimal devnet chain-state skeleton.
//!
//! This is not a consensus freeze. It provides deterministic state transitions
//! that can later be exercised by a real node.

use std::collections::HashSet;

use crate::{
    block::BlockHeader,
    hash::Hash32,
    transaction::{OutPoint, Transaction},
    utxo::{validate_transaction, UtxoEntry, UtxoSet},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    EmptyBlock,
    InvalidTransaction(crate::utxo::ValidationError),
    DuplicateTransaction,
    InvalidPreviousBlock,
    HeightMismatch,
}

#[derive(Debug, Clone, Default)]
pub struct ChainState {
    pub tip: Option<Hash32>,
    pub height: Option<u64>,
    pub utxos: UtxoSet,
}

impl ChainState {
    pub fn apply_block(&mut self, block: &Block) -> Result<Hash32, ChainError> {
        if block.transactions.is_empty() {
            return Err(ChainError::EmptyBlock);
        }

        if let (Some(tip), Some(height)) = (self.tip, self.height) {
            if block.header.previous_block != tip {
                return Err(ChainError::InvalidPreviousBlock);
            }
            if block.header.height.0 != height + 1 {
                return Err(ChainError::HeightMismatch);
            }
        }

        let mut txids = HashSet::with_capacity(block.transactions.len());
        for tx in &block.transactions {
            let txid = tx
                .txid()
                .map_err(|_| ChainError::DuplicateTransaction)?;
            if !txids.insert(txid) {
                return Err(ChainError::DuplicateTransaction);
            }
            validate_transaction(tx, &self.utxos)
                .map_err(ChainError::InvalidTransaction)?;
        }

        for tx in &block.transactions {
            for input in &tx.inputs {
                self.utxos.remove(&input.previous_output);
            }

            let txid = tx
                .txid()
                .map_err(|_| ChainError::DuplicateTransaction)?;

            for (index, output) in tx.outputs.iter().enumerate() {
                self.utxos.insert(
                    OutPoint {
                        txid,
                        index: index as u32,
                    },
                    UtxoEntry {
                        value: output.value,
                        spending_condition: output.spending_condition.clone(),
                    },
                );
            }
        }

        let id = block
            .header
            .block_id()
            .map_err(|_| ChainError::DuplicateTransaction)?;

        self.tip = Some(id);
        self.height = Some(block.header.height.0);

        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_block_is_rejected() {
        let header = BlockHeader {
            version: 1,
            previous_block: Hash32::ZERO,
            height: crate::BlockHeight(0),
            timestamp: 0,
            target: u64::MAX,
            poarm_version: 0,
            poarm_nonce: 0,
            transaction_root: Hash32::ZERO,
        };

        assert_eq!(
            ChainState::default().apply_block(&Block {
                header,
                transactions: vec![],
            }),
            Err(ChainError::EmptyBlock)
        );
    }
}
