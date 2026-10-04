//! Minimal devnet chain-state skeleton.
//!
//! This is not a consensus freeze. It provides deterministic state transitions
//! that can later be exercised by a real node.

use std::collections::HashSet;

use crate::{
    block::BlockHeader,
    hash::Hash32,
    transaction::{OutPoint, Transaction},
    utxo::{UtxoEntry, UtxoSet, validate_transaction},
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
    Serialization(crate::ProtocolError),
    InvalidPreviousBlock,
    HeightMismatch,
    InvalidTransactionRoot,
    InvalidHeader(crate::validation::BlockValidationError),
}

#[derive(Debug, Clone, Default)]
pub struct ChainState {
    pub tip: Option<Hash32>,
    pub height: Option<u64>,
    pub utxos: UtxoSet,
}

impl ChainState {
    /// Validate header-level proof rules and then apply UTXO state transitions.
    pub fn apply_validated_block(
        &mut self,
        block: &Block,
        proof: Hash32,
    ) -> Result<Hash32, ChainError> {
        crate::validation::validate_block_header(self, block, proof)
            .map_err(ChainError::InvalidHeader)?;
        self.apply_block(block)
    }

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

        let expected_root = crate::block::transaction_root(&block.transactions)
            .map_err(ChainError::Serialization)?;
        if block.header.transaction_root != expected_root {
            return Err(ChainError::InvalidTransactionRoot);
        }

        let mut txids = HashSet::with_capacity(block.transactions.len());
        let mut staged_utxos = self.utxos.clone();

        for tx in &block.transactions {
            let txid = tx.txid().map_err(ChainError::Serialization)?;
            if !txids.insert(txid) {
                return Err(ChainError::DuplicateTransaction);
            }

            if !tx.is_coinbase() {
                validate_transaction(tx, &staged_utxos).map_err(ChainError::InvalidTransaction)?;
            }

            for input in &tx.inputs {
                staged_utxos.remove(&input.previous_output);
            }

            for (index, output) in tx.outputs.iter().enumerate() {
                let index = u32::try_from(index)
                    .map_err(|_| ChainError::Serialization(crate::ProtocolError::LengthOverflow))?;
                staged_utxos.insert(
                    OutPoint { txid, index },
                    UtxoEntry {
                        value: output.value,
                        spending_condition: output.spending_condition.clone(),
                    },
                );
            }
        }

        let id = block.header.block_id().map_err(ChainError::Serialization)?;

        self.utxos = staged_utxos;
        self.tip = Some(id);
        self.height = Some(block.header.height.0);

        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(height: u64, previous_block: Hash32) -> BlockHeader {
        BlockHeader {
            version: 1,
            previous_block,
            height: crate::BlockHeight(height),
            timestamp: height,
            target: u64::MAX,
            poarm_version: 0,
            poarm_nonce: height,
            transaction_root: Hash32::ZERO,
        }
    }

    fn spend(previous_output: OutPoint, value: u64, output_condition: &[u8]) -> Transaction {
        Transaction {
            version: 1,
            inputs: vec![crate::TxInput {
                previous_output,
                unlocking_data: Vec::new(),
            }],
            outputs: vec![crate::TxOutput {
                value,
                spending_condition: output_condition.to_vec(),
            }],
            fee: 0,
        }
    }

    #[test]
    fn empty_block_is_rejected() {
        assert_eq!(
            ChainState::default().apply_block(&Block {
                header: header(0, Hash32::ZERO),
                transactions: vec![],
            }),
            Err(ChainError::EmptyBlock)
        );
    }

    #[test]
    fn transactions_can_spend_outputs_created_earlier_in_the_block() {
        let mut state = ChainState::default();
        state.utxos.insert(
            OutPoint {
                txid: Hash32([8u8; 32]),
                index: 0,
            },
            UtxoEntry {
                value: 100,
                spending_condition: b"funding".to_vec(),
            },
        );

        let funding = spend(
            OutPoint {
                txid: Hash32([8u8; 32]),
                index: 0,
            },
            100,
            b"seed",
        );
        let funding_id = funding.txid().unwrap();
        let expected_seed_outpoint = OutPoint {
            txid: funding_id,
            index: 0,
        };

        let follow_up = spend(expected_seed_outpoint, 100, b"final");

        let follow_up_id = follow_up.txid().unwrap();
        let transactions = vec![funding, follow_up];
        let mut block_header = header(0, Hash32::ZERO);
        block_header.transaction_root = crate::block::transaction_root(&transactions).unwrap();

        state
            .apply_block(&Block {
                header: block_header,
                transactions,
            })
            .unwrap();

        assert!(state.utxos.contains_key(&OutPoint {
            txid: follow_up_id,
            index: 0,
        }));
    }
}
