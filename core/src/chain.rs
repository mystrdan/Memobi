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
    TooManyTransactions,
    BlockTooLarge,
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
    /// Full header history for reorg/difficulty decisions (devnet scale).
    pub headers: Vec<BlockHeader>,
    /// Cumulative work of the canonical chain.
    pub work: crate::chainwork::WorkScore,
}

impl ChainState {
    /// Validate header-level proof rules and then apply UTXO state transitions.
    pub fn apply_validated_block(
        &mut self,
        block: &Block,
        proof: Hash32,
    ) -> Result<Hash32, ChainError> {
        self.apply_validated_block_with_context(block, proof, block.header.timestamp, None)
    }

    pub fn apply_validated_block_with_context(
        &mut self,
        block: &Block,
        proof: Hash32,
        now_secs: u64,
        parent_timestamp: Option<u64>,
    ) -> Result<Hash32, ChainError> {
        let params = crate::params::ConsensusParams::devnet();
        self.apply_validated_block_with_params_and_context(
            block,
            proof,
            &params,
            now_secs,
            parent_timestamp,
        )
    }

    /// Validate and apply a block using an explicit network parameter set.
    ///
    /// This is the parameterized consensus entry point. The convenience
    /// methods above remain devnet defaults for existing callers.
    pub fn apply_validated_block_with_params_and_context(
        &mut self,
        block: &Block,
        proof: Hash32,
        params: &crate::params::ConsensusParams,
        now_secs: u64,
        parent_timestamp: Option<u64>,
    ) -> Result<Hash32, ChainError> {
        let parent = parent_timestamp.or_else(|| self.headers.last().map(|h| h.timestamp));
        crate::validation::validate_block_header_with_params(
            self, block, proof, params, now_secs, parent,
        )
        .map_err(ChainError::InvalidHeader)?;
        self.apply_block_with_params(block, params)
    }

    pub fn apply_block(&mut self, block: &Block) -> Result<Hash32, ChainError> {
        let id = self.apply_block_with_params(block, &crate::params::ConsensusParams::devnet())?;
        // `apply_block_with_params` stages state but defers tip/work commit
        // to this wrapper so direct callers share one commit path.
        // (No-op here: commit already done inside; kept for clarity.)
        Ok(id)
    }

    pub fn apply_block_with_params(
        &mut self,
        block: &Block,
        params: &crate::params::ConsensusParams,
    ) -> Result<Hash32, ChainError> {
        if block.transactions.is_empty() {
            return Err(ChainError::EmptyBlock);
        }
        if block.transactions.len() > params.max_txs_per_block {
            return Err(ChainError::TooManyTransactions);
        }
        // Block size bound: header 104B + sum of tx encodings.
        let mut block_bytes = 104usize;
        for tx in &block.transactions {
            let enc = tx.encode_to_vec().map_err(ChainError::Serialization)?;
            block_bytes = block_bytes
                .checked_add(enc.len())
                .ok_or(ChainError::Serialization(
                    crate::ProtocolError::LengthOverflow,
                ))?;
        }
        if block_bytes > params.max_block_bytes {
            return Err(ChainError::BlockTooLarge);
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
        let height = block.header.height.0;

        for (tx_index, tx) in block.transactions.iter().enumerate() {
            let txid = tx.txid().map_err(ChainError::Serialization)?;
            if !txids.insert(txid) {
                return Err(ChainError::DuplicateTransaction);
            }

            if tx_index == 0 && height != 0 {
                // Coinbase position and marker semantics are enforced by the
                // validation layer; skip ordinary spend checks here.
            } else if tx_index == 0 && height == 0 {
                // Genesis is a special zero-input identity transaction.
            } else if !tx.is_coinbase() {
                validate_transaction(tx, &staged_utxos, height, params)
                    .map_err(ChainError::InvalidTransaction)?;
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
                        height,
                        is_coinbase: tx_index == 0 && height != 0,
                    },
                );
            }
        }

        let id = block.header.block_id().map_err(ChainError::Serialization)?;

        self.utxos = staged_utxos;
        self.tip = Some(id);
        self.height = Some(block.header.height.0);
        self.headers.push(block.header);
        self.work = crate::chainwork::WorkScore(
            self.work
                .0
                .saturating_add(crate::chainwork::block_work(block.header.target).0),
        );

        Ok(id)
    }

    /// Reorg support: compare a candidate fork (built from the same genesis)
    /// by cumulative work. Returns true if the fork should replace `self`.
    /// Callers rebuild state by replaying the winning fork; this helper only
    /// decides, keeping state mutation explicit and auditable.
    pub fn should_reorg(&self, fork_work: crate::chainwork::WorkScore) -> bool {
        crate::chainwork::prefers_chain(fork_work, self.work)
    }

    pub fn timestamps(&self) -> Vec<u64> {
        self.headers.iter().map(|h| h.timestamp).collect()
    }

    /// Execute a reorg by fully replaying `fork`: blocks from genesis (oldest
    /// first), each paired with its PoW proof.
    ///
    /// The fork is validated in a scratch state — header context, difficulty,
    /// coinbase, transactions — so any error leaves `self` unchanged (atomic).
    /// The fork is committed only if it wins: strictly more cumulative work,
    /// or equal work with more blocks (trivial devnet targets contribute zero
    /// work, so ties are broken by length there; production targets are never
    /// `u64::MAX`). Returns `Ok(true)` when the reorg executed. The fork must
    /// start at genesis (`height == 0`, `previous == 0`), which replay enforces.
    pub fn replay_fork(&mut self, fork: &[(Block, Hash32)]) -> Result<bool, ChainError> {
        self.replay_fork_with_params(fork, &crate::params::ConsensusParams::devnet())
    }

    /// Replay and conditionally commit a fork using an explicit network
    /// parameter set. Validation happens entirely in a scratch state, so a
    /// malformed or non-winning fork leaves the current chain untouched.
    pub fn replay_fork_with_params(
        &mut self,
        fork: &[(Block, Hash32)],
        params: &crate::params::ConsensusParams,
    ) -> Result<bool, ChainError> {
        let mut candidate = ChainState::default();
        for (block, proof) in fork {
            candidate.apply_validated_block_with_params_and_context(
                block,
                *proof,
                params,
                block.header.timestamp,
                None,
            )?;
        }
        if candidate.headers.is_empty() {
            return Ok(false);
        }
        let wins = self.headers.is_empty()
            || self.should_reorg(candidate.work)
            || (candidate.work == self.work && candidate.headers.len() > self.headers.len());
        if wins {
            *self = candidate;
        }
        Ok(wins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::ConsensusParams;

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

    fn fund(state: &mut ChainState, sk: &crate::crypto::SecretKey, value: u64) -> OutPoint {
        let op = OutPoint {
            txid: Hash32([8u8; 32]),
            index: 0,
        };
        state.utxos.insert(
            op,
            UtxoEntry {
                value,
                spending_condition: sk.public_key().to_vec(),
                height: 0,
                is_coinbase: false,
            },
        );
        op
    }

    fn signed_spend(
        previous_output: OutPoint,
        value: u64,
        sk: &crate::crypto::SecretKey,
        params: &ConsensusParams,
    ) -> Transaction {
        let mut tx = Transaction {
            version: 1,
            inputs: vec![crate::TxInput {
                previous_output,
                unlocking_data: Vec::new(),
            }],
            outputs: vec![crate::TxOutput {
                value,
                spending_condition: sk.public_key().to_vec(),
            }],
            fee: 0,
        };
        tx.inputs[0].unlocking_data = crate::crypto::authorize_input(&tx, 0, sk, params).unwrap();
        tx
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
    fn heavier_fork_wins_reorg_decision() {
        let easy = crate::chainwork::cumulative_work([u64::MAX, u64::MAX]);
        let hard = crate::chainwork::cumulative_work([100, 100]);
        let state = ChainState {
            work: easy,
            ..Default::default()
        };
        assert!(state.should_reorg(hard));
        assert!(!state.should_reorg(easy));
    }

    #[test]
    fn parameterized_fork_replay_uses_supplied_consensus_params() {
        let mut params = ConsensusParams::devnet();
        params.tx_version = 7;
        params.poarm_domain = "MEMOBI-POARM-TEST";

        let genesis = crate::genesis::devnet_genesis();
        let mut candidate = ChainState::default();
        candidate.apply_block(&genesis).unwrap();

        let miner = crate::crypto::SecretKey::from_bytes(&[19u8; 32]).public_key();
        let template = crate::block_builder::BlockTemplate {
            version: params.block_version,
            previous_block: candidate.tip.unwrap(),
            height: BlockHeight(1),
            timestamp: 10,
            target: params.max_target,
            poarm_version: params.poarm_version,
            poarm_nonce: 0,
        };
        let block = template
            .build_mining_block_with_params(Vec::new(), miner.to_vec(), &params)
            .unwrap();
        assert_eq!(block.transactions[0].version, params.tx_version);

        let fork = vec![(genesis, Hash32::ZERO), (block, Hash32::ZERO)];
        let mut state = ChainState::default();
        assert!(state.replay_fork_with_params(&fork, &params).unwrap());
        assert_eq!(state.height, Some(1));
    }

    #[test]
    fn transactions_can_spend_outputs_created_earlier_in_the_block() {
        let params = ConsensusParams::devnet();
        let sk = crate::crypto::SecretKey::from_bytes(&[8u8; 32]);
        let mut state = ChainState::default();
        let funding_op = fund(&mut state, &sk, 100);

        let funding = signed_spend(funding_op, 100, &sk, &params);
        let funding_id = funding.txid().unwrap();
        let expected_seed_outpoint = OutPoint {
            txid: funding_id,
            index: 0,
        };

        let follow_up = signed_spend(expected_seed_outpoint, 100, &sk, &params);

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
