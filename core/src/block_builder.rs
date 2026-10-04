//! Provisional devnet block-construction helpers.
//!
//! This builder assembles deterministic block headers and transaction roots.
//! Coinbase/reward semantics, timestamp rules, and PoARM proof validation are
//! intentionally left outside this builder until consensus is frozen.

use crate::{
    BlockHeight, Hash32, ProtocolError,
    block::{BlockHeader, transaction_root},
    chain::Block,
    transaction::Transaction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockTemplate {
    pub version: u32,
    pub previous_block: Hash32,
    pub height: BlockHeight,
    pub timestamp: u64,
    pub target: u64,
    pub poarm_version: u32,
    pub poarm_nonce: u64,
}

impl BlockTemplate {
    /// Build a provisional mining block with a coinbase payout.
    /// The supplied transactions must be ordinary transactions.
    pub fn build_mining_block(
        self,
        transactions: Vec<Transaction>,
        payout_condition: Vec<u8>,
        reward_config: crate::reward::RewardConfig,
    ) -> Result<Block, ProtocolError> {
        let fees = transactions.iter().try_fold(0u64, |sum, tx| {
            sum.checked_add(tx.fee).ok_or(ProtocolError::LengthOverflow)
        })?;
        let subsidy = crate::reward::block_subsidy(self.height.0, reward_config);
        let payout = subsidy.checked_add(fees).ok_or(ProtocolError::LengthOverflow)?;
        let coinbase = Transaction {
            version: 1,
            inputs: Vec::new(),
            outputs: vec![crate::TxOutput {
                value: payout,
                spending_condition: payout_condition,
            }],
            fee: 0,
        };
        let mut all = Vec::with_capacity(transactions.len() + 1);
        all.push(coinbase);
        all.extend(transactions);
        self.build(all)
    }

    pub fn build(self, transactions: Vec<Transaction>) -> Result<Block, ProtocolError> {
        let transaction_root = transaction_root(&transactions)?;
        Ok(Block {
            header: BlockHeader {
                version: self.version,
                previous_block: self.previous_block,
                height: self.height,
                timestamp: self.timestamp,
                target: self.target,
                poarm_version: self.poarm_version,
                poarm_nonce: self.poarm_nonce,
                transaction_root,
            },
            transactions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_builds_deterministic_header_root() {
        let tx = Transaction {
            version: 1,
            inputs: vec![crate::TxInput {
                previous_output: crate::OutPoint {
                    txid: Hash32([7; 32]),
                    index: 0,
                },
                unlocking_data: b"sig".to_vec(),
            }],
            outputs: vec![crate::TxOutput {
                value: 10,
                spending_condition: b"condition".to_vec(),
            }],
            fee: 0,
        };
        let template = BlockTemplate {
            version: 1,
            previous_block: Hash32([1; 32]),
            height: BlockHeight(1),
            timestamp: 10,
            target: u64::MAX,
            poarm_version: 0,
            poarm_nonce: 2,
        };
        let block = template.build(vec![tx]).unwrap();
        assert_eq!(block.header.height, BlockHeight(1));
        assert_eq!(
            block.header.transaction_root,
            transaction_root(&block.transactions).unwrap()
        );
    }
}
