//! Block validation: structural, contextual, and proof checks.
//!
//! Invalid blocks never mutate state; this module only inspects.

use crate::{
    Hash32,
    chain::{Block, ChainState},
    pow::{PowTarget, meets_target},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockValidationError {
    EmptyBlock,
    TooManyTransactions,
    BlockTooLarge,
    InvalidHeight,
    InvalidPreviousBlock,
    InvalidTransactionRoot,
    InvalidProof,
    InvalidTarget,
    InvalidDifficulty,
    UnsupportedBlockVersion,
    UnsupportedPoarmVersion,
    TimestampTooOld,
    TimestampTooFar,
    MissingCoinbase,
    MultipleCoinbase,
    InvalidCoinbase,
    RewardOverflow,
    ExcessiveReward,
    InvalidTransaction,
}

fn state_tip_target_opt(state: &ChainState) -> Option<u64> {
    state.headers.last().map(|h| h.target)
}

fn validate_coinbase(
    block: &Block,
    state: &ChainState,
    params: &crate::params::ConsensusParams,
) -> Result<(), BlockValidationError> {
    let Some((first, rest)) = block.transactions.split_first() else {
        return Err(BlockValidationError::MissingCoinbase);
    };
    if block.header.height.0 == 0 {
        return Ok(());
    }
    if !first.is_coinbase() {
        return Err(BlockValidationError::MissingCoinbase);
    }
    if first.coinbase_height() != Some(block.header.height.0) {
        return Err(BlockValidationError::InvalidCoinbase);
    }
    if rest.iter().any(crate::Transaction::is_coinbase) {
        return Err(BlockValidationError::MultipleCoinbase);
    }
    if first.version != params.tx_version {
        return Err(BlockValidationError::InvalidCoinbase);
    }
    if first.fee != 0 || first.outputs.iter().any(|output| output.value == 0) {
        return Err(BlockValidationError::InvalidCoinbase);
    }
    if first.outputs.len() > params.max_outputs_per_tx {
        return Err(BlockValidationError::InvalidCoinbase);
    }
    for output in &first.outputs {
        if output.spending_condition.len() != crate::crypto::PUBKEY_LEN {
            return Err(BlockValidationError::InvalidCoinbase);
        }
        if output.value > params.max_money {
            return Err(BlockValidationError::RewardOverflow);
        }
    }

    let mut staged = state.utxos.clone();
    let mut fees = 0u64;
    for tx in rest {
        crate::utxo::validate_transaction(tx, &staged, block.header.height.0, params)
            .map_err(|_| BlockValidationError::InvalidCoinbase)?;
        fees = fees
            .checked_add(tx.fee)
            .ok_or(BlockValidationError::RewardOverflow)?;
        for input in &tx.inputs {
            staged.remove(&input.previous_output);
        }
        let txid = tx
            .txid()
            .map_err(|_| BlockValidationError::RewardOverflow)?;
        for (index, output) in tx.outputs.iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| BlockValidationError::RewardOverflow)?;
            staged.insert(
                crate::OutPoint { txid, index },
                crate::utxo::UtxoEntry {
                    value: output.value,
                    spending_condition: output.spending_condition.clone(),
                    height: block.header.height.0,
                    is_coinbase: false,
                },
            );
        }
    }

    let subsidy = crate::reward::block_subsidy(
        block.header.height.0,
        crate::reward::RewardConfig {
            initial_reward: params.initial_reward,
            halving_interval: params.halving_interval,
        },
    );
    let allowed = subsidy
        .checked_add(fees)
        .ok_or(BlockValidationError::RewardOverflow)?;
    let coinbase_value = first.outputs.iter().try_fold(0u64, |sum, output| {
        sum.checked_add(output.value)
            .ok_or(BlockValidationError::RewardOverflow)
    })?;
    if coinbase_value > allowed {
        return Err(BlockValidationError::ExcessiveReward);
    }
    Ok(())
}
pub fn validate_block_header(
    state: &ChainState,
    block: &Block,
    proof: Hash32,
) -> Result<(), BlockValidationError> {
    validate_block_header_with_params(
        state,
        block,
        proof,
        &crate::params::ConsensusParams::devnet(),
        0,
        None,
    )
}

/// Full contextual validation. `parent_timestamp` is the previous header time;
/// `median_past` is the median of the last 11 (or fewer) timestamps.
pub fn validate_block_header_with_params(
    state: &ChainState,
    block: &Block,
    proof: Hash32,
    params: &crate::params::ConsensusParams,
    now_secs: u64,
    parent_timestamp: Option<u64>,
) -> Result<(), BlockValidationError> {
    if block.transactions.is_empty() {
        return Err(BlockValidationError::EmptyBlock);
    }
    if block.transactions.len() > params.max_txs_per_block {
        return Err(BlockValidationError::TooManyTransactions);
    }
    if block.header.version != params.block_version {
        return Err(BlockValidationError::UnsupportedBlockVersion);
    }
    if block.header.poarm_version != params.poarm_version {
        return Err(BlockValidationError::UnsupportedPoarmVersion);
    }
    if block.header.target < params.min_target || block.header.target > params.max_target {
        return Err(BlockValidationError::InvalidTarget);
    }
    // Timestamp: strictly increasing vs parent; bounded future skew.
    if let Some(parent) = parent_timestamp
        && block.header.timestamp <= parent
    {
        return Err(BlockValidationError::TimestampTooOld);
    }
    if block.header.timestamp > now_secs.saturating_add(params.max_future_skew_secs) {
        return Err(BlockValidationError::TimestampTooFar);
    }
    // Expected difficulty derived deterministically from chain history so
    // every node computes the same target for identical timestamps.
    // With no header history the candidate must be genesis (pinned target);
    // a non-genesis block on empty state is rejected by height checks below.
    if !state.headers.is_empty() || block.header.height.0 == 0 {
        let history = state.timestamps();
        let expected = crate::difficulty::next_block_target(
            &history,
            state_tip_target_opt(state),
            block.header.timestamp,
            params,
        )
        .map_err(|_| BlockValidationError::InvalidDifficulty)?;
        if block.header.target != expected {
            return Err(BlockValidationError::InvalidDifficulty);
        }
    }
    validate_coinbase(block, state, params)?;
    // Every non-coinbase tx must fully validate (sigs, maturity, money).
    let height = block.header.height.0;
    {
        let mut staged = state.utxos.clone();
        for (i, tx) in block.transactions.iter().enumerate() {
            if i == 0 && height != 0 {
                continue;
            }
            if tx.is_coinbase() && height == 0 {
                continue;
            }
            crate::utxo::validate_transaction(tx, &staged, height, params)
                .map_err(|_| BlockValidationError::InvalidTransaction)?;
            for input in &tx.inputs {
                staged.remove(&input.previous_output);
            }
            let txid = tx
                .txid()
                .map_err(|_| BlockValidationError::RewardOverflow)?;
            for (index, output) in tx.outputs.iter().enumerate() {
                let index =
                    u32::try_from(index).map_err(|_| BlockValidationError::RewardOverflow)?;
                staged.insert(
                    crate::OutPoint { txid, index },
                    crate::utxo::UtxoEntry {
                        value: output.value,
                        spending_condition: output.spending_condition.clone(),
                        height,
                        is_coinbase: false,
                    },
                );
            }
        }
    }
    let expected_root = crate::block::transaction_root(&block.transactions)
        .map_err(|_| BlockValidationError::InvalidTransactionRoot)?;
    if expected_root != block.header.transaction_root {
        return Err(BlockValidationError::InvalidTransactionRoot);
    }
    if let (Some(tip), Some(tip_height)) = (state.tip, state.height) {
        if block.header.previous_block != tip {
            return Err(BlockValidationError::InvalidPreviousBlock);
        }
        if block.header.height.0 != tip_height + 1 {
            return Err(BlockValidationError::InvalidHeight);
        }
    } else if block.header.height.0 != 0 || block.header.previous_block != Hash32::ZERO {
        return Err(BlockValidationError::InvalidHeight);
    }
    if !meets_target(proof, PowTarget(block.header.target)) {
        return Err(BlockValidationError::InvalidProof);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockHeight, Transaction, TxOutput};
    fn block() -> Block {
        // Height-0 genesis-style block: coinbase bypass permitted.
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOutput {
                value: 1,
                spending_condition: vec![0u8; 32],
            }],
            fee: 0,
        };
        let root = crate::block::transaction_root(std::slice::from_ref(&tx)).unwrap();
        Block {
            header: crate::block::BlockHeader {
                version: 1,
                previous_block: Hash32::ZERO,
                height: BlockHeight(0),
                timestamp: 0,
                target: u64::MAX,
                poarm_version: 0,
                poarm_nonce: 0,
                transaction_root: root,
            },
            transactions: vec![tx],
        }
    }
    #[test]
    fn valid_devnet_header_passes() {
        assert!(validate_block_header(&ChainState::default(), &block(), Hash32::ZERO).is_ok());
    }
    #[test]
    fn zero_target_is_rejected() {
        let mut b = block();
        b.header.target = 0;
        assert_eq!(
            validate_block_header(&ChainState::default(), &b, Hash32::ZERO),
            Err(BlockValidationError::InvalidTarget)
        );
    }
    #[test]
    fn failed_proof_is_rejected() {
        let mut b = block();
        // Height 1 enforces coinbase position + reward, so build valid coinbase.
        b.header.height = BlockHeight(1);
        b.header.previous_block = Hash32::ZERO;
        // ChainState default has no tip, so set height 0 expectation manually:
        // use a state with tip ZERO at height 0.
        let state = ChainState {
            tip: Some(Hash32::ZERO),
            height: Some(0),
            ..Default::default()
        };
        b.header.target = 10;
        // Recompute root after no tx change (same single coinbase).
        let mut p = [0; 32];
        p[..8].copy_from_slice(&11u64.to_be_bytes());
        assert_eq!(
            validate_block_header(&state, &b, Hash32(p)),
            Err(BlockValidationError::InvalidProof)
        );
    }
}
