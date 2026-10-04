//! Provisional block-validation boundary.
//!
//! This module validates rules that can be expressed independently of the
//! concrete PoARM implementation. PoARM itself supplies a 32-byte proof and
//! this layer checks that proof against the header target.

use crate::{Hash32, block::Block, chain::ChainState, pow::{PowTarget, meets_target}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockValidationError {
    EmptyBlock, InvalidHeight, InvalidPreviousBlock, InvalidTransactionRoot,
    InvalidProof, InvalidTarget,
}

pub fn validate_block_header(state: &ChainState, block: &Block, proof: Hash32) -> Result<(), BlockValidationError> {
    if block.transactions.is_empty() { return Err(BlockValidationError::EmptyBlock); }
    if block.header.target == 0 { return Err(BlockValidationError::InvalidTarget); }
    let expected_root = crate::block::transaction_root(&block.transactions)
        .map_err(|_| BlockValidationError::InvalidTransactionRoot)?;
    if expected_root != block.header.transaction_root { return Err(BlockValidationError::InvalidTransactionRoot); }
    if let (Some(tip), Some(height)) = (state.tip, state.height) {
        if block.header.previous_block != tip { return Err(BlockValidationError::InvalidPreviousBlock); }
        if block.header.height.0 != height + 1 { return Err(BlockValidationError::InvalidHeight); }
    } else if block.header.height.0 != 0 || block.header.previous_block != Hash32::ZERO {
        return Err(BlockValidationError::InvalidHeight);
    }
    if !meets_target(proof, PowTarget(block.header.target)) { return Err(BlockValidationError::InvalidProof); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockHeight, TxInput, TxOutput, Transaction};
    fn block() -> Block {
        let tx = Transaction { version: 1, inputs: vec![TxInput { previous_output: crate::OutPoint { txid: Hash32([3; 32]), index: 0 }, unlocking_data: vec![] }], outputs: vec![TxOutput { value: 1, spending_condition: vec![] }], fee: 0 };
        let root = crate::block::transaction_root(std::slice::from_ref(&tx)).unwrap();
        Block { header: crate::block::BlockHeader { version: 1, previous_block: Hash32::ZERO, height: BlockHeight(0), timestamp: 0, target: u64::MAX, poarm_version: 0, poarm_nonce: 0, transaction_root: root }, transactions: vec![tx] }
    }
    #[test] fn valid_devnet_header_passes() { assert!(validate_block_header(&ChainState::default(), &block(), Hash32::ZERO).is_ok()); }
    #[test] fn zero_target_is_rejected() { let mut b=block(); b.header.target=0; assert_eq!(validate_block_header(&ChainState::default(), &b, Hash32::ZERO), Err(BlockValidationError::InvalidTarget)); }
    #[test] fn failed_proof_is_rejected() { let mut b=block(); b.header.target=10; let mut p=[0;32]; p[..8].copy_from_slice(&11u64.to_be_bytes()); assert_eq!(validate_block_header(&ChainState::default(), &b, Hash32(p)), Err(BlockValidationError::InvalidProof)); }
}