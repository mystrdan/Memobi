//! Deterministic devnet chain producer built on the real consensus path.
//!
//! This is a development harness, not a second consensus implementation.
//! Every produced block is built, mined, independently verified, validated,
//! and applied through the same ChainState path used by consensus tests.

use memobi_core::{
    Hash32,
    block_builder::BlockTemplate,
    chain::{Block, ChainError, ChainState},
    crypto::SecretKey,
    genesis::devnet_genesis,
    params::ConsensusParams,
};

use crate::{
    Config,
    miner::{MiningResult, search_candidate_c, verify_candidate_c},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevnetProducerConfig {
    /// Number of post-genesis blocks to produce.
    pub blocks: u64,
    /// Deterministic timestamp increment for each block.
    pub timestamp_step: u64,
    /// PoARM epoch supplied to the seed/miner.
    pub epoch: u64,
    /// Experimental PoARM configuration used by the devnet harness.
    pub poarm: Config,
    /// Maximum nonce attempts per block.
    pub max_attempts: u64,
    /// Deterministic 32-byte private key used only by the devnet payout.
    pub payout_seed: [u8; 32],
}

impl Default for DevnetProducerConfig {
    fn default() -> Self {
        Self {
            blocks: 10,
            timestamp_step: ConsensusParams::devnet().target_interval_secs,
            epoch: 0,
            poarm: Config {
                memory_kib: 1,
                rounds: 1,
            },
            max_attempts: 10_000,
            payout_seed: [42u8; 32],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducedBlock {
    pub block: Block,
    pub proof: Hash32,
    pub mining: MiningResult,
}

#[derive(Debug, Clone)]
pub struct DevnetResult {
    pub chain: ChainState,
    pub blocks: Vec<ProducedBlock>,
}

#[derive(Debug)]
pub enum DevnetError {
    Chain(ChainError),
    MiningExhausted { height: u64 },
    InvalidProof { height: u64 },
    InvalidSeed,
    BlockBuild(memobi_core::ProtocolError),
}

impl From<ChainError> for DevnetError {
    fn from(value: ChainError) -> Self {
        Self::Chain(value)
    }
}

impl From<memobi_core::ProtocolError> for DevnetError {
    fn from(value: memobi_core::ProtocolError) -> Self {
        Self::BlockBuild(value)
    }
}

/// Produce a deterministic devnet chain using the existing consensus path.
pub fn produce(config: DevnetProducerConfig) -> Result<DevnetResult, DevnetError> {
    let params = ConsensusParams::devnet();
    let mut chain = ChainState::default();
    let genesis = devnet_genesis();
    let genesis_ts = genesis.header.timestamp;
    chain.apply_block(&genesis)?;

    let payout = SecretKey::from_bytes(&config.payout_seed)
        .public_key()
        .to_vec();
    let mut produced = Vec::with_capacity(config.blocks as usize);

    for offset in 0..config.blocks {
        let parent = chain.tip.ok_or(DevnetError::InvalidSeed)?;
        let height = chain.height.ok_or(DevnetError::InvalidSeed)? + 1;
        let timestamp = genesis_ts
            .checked_add(
                config
                    .timestamp_step
                    .checked_mul(offset + 1)
                    .ok_or(DevnetError::InvalidSeed)?,
            )
            .ok_or(DevnetError::InvalidSeed)?;

        let target = memobi_core::difficulty::next_block_target(
            &chain.timestamps(),
            chain.headers.last().map(|h| h.target),
            timestamp,
            &params,
        )
        .map_err(|_| DevnetError::InvalidSeed)?;

        let template = BlockTemplate {
            version: params.block_version,
            previous_block: parent,
            height: memobi_core::BlockHeight(height),
            timestamp,
            target,
            poarm_version: params.poarm_version,
            poarm_nonce: 0,
        };

        let mut block = template.build_mining_block_with_params(
            Vec::new(),
            payout.clone(),
            &params,
        )?;

        let seed = block
            .header
            .poarm_seed_with_params(&params, config.epoch)
            .map_err(|_| DevnetError::InvalidSeed)?;

        let mining = search_candidate_c(
            seed.as_bytes(),
            config.epoch,
            config.poarm,
            block.header.target,
            0,
            config.max_attempts,
        )
        .ok_or(DevnetError::MiningExhausted { height })?;

        if !verify_candidate_c(
            seed.as_bytes(),
            config.epoch,
            config.poarm,
            block.header.target,
            mining.nonce,
            &mining.proof,
        ) {
            return Err(DevnetError::InvalidProof { height });
        }

        block.header.poarm_nonce = mining.nonce;

        let proof = Hash32(mining.proof);
        chain.apply_validated_block_with_params_and_context(
            &block,
            proof,
            &params,
            timestamp,
            Some(timestamp - config.timestamp_step),
        )?;

        produced.push(ProducedBlock {
            block,
            proof,
            mining,
        });
    }

    Ok(DevnetResult {
        chain,
        blocks: produced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_multiple_real_consensus_blocks() {
        let result = produce(DevnetProducerConfig {
            blocks: 10,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(result.chain.height, Some(10));
        assert_eq!(result.blocks.len(), 10);

        for pair in result.blocks.windows(2) {
            assert_eq!(
                pair[1].block.header.previous_block,
                pair[0].block.header.block_id().unwrap()
            );
            assert_eq!(
                pair[1].block.header.height.0,
                pair[0].block.header.height.0 + 1
            );
        }
    }

    #[test]
    fn production_is_deterministic() {
        let a = produce(DevnetProducerConfig {
            blocks: 5,
            ..Default::default()
        })
        .unwrap();
        let b = produce(DevnetProducerConfig {
            blocks: 5,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(a.chain.tip, b.chain.tip);
        assert_eq!(a.chain.height, b.chain.height);
        assert_eq!(
            a.blocks
                .iter()
                .map(|x| x.block.header.block_id().unwrap())
                .collect::<Vec<_>>(),
            b.blocks
                .iter()
                .map(|x| x.block.header.block_id().unwrap())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn target_follows_consensus_difficulty_path() {
        let result = produce(DevnetProducerConfig {
            blocks: 60,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(result.chain.height, Some(60));
        assert_eq!(result.blocks.last().unwrap().block.header.target, u64::MAX);
    }
}
