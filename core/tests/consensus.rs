//! Consensus integration: multi-block production, validation, and reorg.
//!
//! Exercises the full pipeline: genesis -> mine -> validate -> apply,
//! double-spend rejection, timestamp rules, and heaviest-chain reorg.

use memobi_core::{
    BlockHeight, Hash32,
    block_builder::BlockTemplate,
    chain::{Block, ChainState},
    crypto::{SecretKey, authorize_input},
    genesis::devnet_genesis,
    params::ConsensusParams,
};

fn mine_next(
    chain: &ChainState,
    params: &ConsensusParams,
    miner_pk: [u8; 32],
    timestamp: u64,
    target: u64,
) -> (Block, Hash32) {
    let (prev, height) = match (chain.tip, chain.height) {
        (Some(tip), Some(h)) => (tip, h + 1),
        _ => (Hash32::ZERO, 0),
    };
    let template = BlockTemplate {
        version: params.block_version,
        previous_block: prev,
        height: BlockHeight(height),
        timestamp,
        target,
        poarm_version: params.poarm_version,
        poarm_nonce: 0,
    };
    let epoch = template.height.0;
    let mut block = template
        .build_mining_block_with_params(Vec::new(), miner_pk.to_vec(), params)
        .unwrap();
    let seed = block.header.poarm_seed_with_params(params, epoch).unwrap();
    let found = memobi_poarm::miner::search_candidate_c(
        seed.as_bytes(),
        epoch,
        memobi_poarm::Config {
            memory_kib: 1,
            rounds: 1,
        },
        target,
        0,
        100,
    )
    .unwrap();
    block.header.poarm_nonce = found.nonce;
    (block, Hash32(found.proof))
}

#[test]
fn multi_block_chain_accepts_and_tracks_work() {
    let params = ConsensusParams::devnet();
    let miner = SecretKey::from_bytes(&[1u8; 32]).public_key();
    let mut chain = ChainState::default();
    chain.apply_block(&devnet_genesis()).unwrap();
    for i in 1u64..=3 {
        let tip_ts = chain.headers.last().unwrap().timestamp;
        let (block, proof) = mine_next(&chain, &params, miner, tip_ts + 10, u64::MAX);
        chain.apply_validated_block(&block, proof).unwrap();
        assert_eq!(chain.height, Some(i));
    }
    // MAX target => block_work = MAX/(MAX+1) = 0; work stays 0 on easy devnet.
    assert_eq!(chain.headers.len(), 4);
    assert_eq!(chain.work.0, 0);
    // Harder targets contribute work: sanity check scoring separately.
    assert!(
        memobi_core::chainwork::block_work(1).0 > memobi_core::chainwork::block_work(u64::MAX).0
    );
}

#[test]
fn double_spend_in_same_block_is_rejected() {
    let params = ConsensusParams::devnet();
    let miner_sk = SecretKey::from_bytes(&[2u8; 32]);
    let miner = miner_sk.public_key();
    let mut chain = ChainState::default();
    chain.apply_block(&devnet_genesis()).unwrap();
    let (b1, p1) = mine_next(
        &chain,
        &params,
        miner,
        params.genesis_timestamp + 10,
        u64::MAX,
    );
    chain.apply_validated_block(&b1, p1).unwrap();
    for _ in 2u64..=11 {
        let tip_ts = chain.headers.last().unwrap().timestamp;
        let (b, p) = mine_next(&chain, &params, miner, tip_ts + 10, u64::MAX);
        chain.apply_validated_block(&b, p).unwrap();
    }
    let coinbase_txid = b1.transactions[0].txid().unwrap();
    let op = memobi_core::OutPoint {
        txid: coinbase_txid,
        index: 0,
    };
    let mk_spend = |value: u64| {
        let mut tx = memobi_core::Transaction {
            version: 1,
            inputs: vec![memobi_core::TxInput {
                previous_output: op,
                unlocking_data: Vec::new(),
            }],
            outputs: vec![memobi_core::TxOutput {
                value,
                spending_condition: miner.to_vec(),
            }],
            fee: params.initial_reward - value,
        };
        tx.inputs[0].unlocking_data = authorize_input(&tx, 0, &miner_sk, &params).unwrap();
        tx
    };
    let t1 = mk_spend(params.initial_reward - 100);
    let t2 = mk_spend(params.initial_reward - 200);
    let template = BlockTemplate {
        version: 1,
        previous_block: chain.tip.unwrap(),
        height: BlockHeight(12),
        timestamp: chain.headers.last().unwrap().timestamp + 10,
        target: u64::MAX,
        poarm_version: 0,
        poarm_nonce: 0,
    };
    let block = template
        .build_mining_block(
            vec![t1, t2],
            miner.to_vec(),
            memobi_core::reward::RewardConfig {
                initial_reward: params.initial_reward,
                halving_interval: params.halving_interval,
            },
        )
        .unwrap();
    assert!(
        chain
            .apply_validated_block(&block, Hash32([0u8; 32]))
            .is_err()
    );
    assert_eq!(chain.height, Some(11));
}

#[test]
fn timestamp_and_target_rules_are_enforced() {
    use memobi_core::validation::{BlockValidationError, validate_block_header_with_params};
    let params = ConsensusParams::devnet();
    let miner = SecretKey::from_bytes(&[3u8; 32]).public_key();
    let mut chain = ChainState::default();
    chain.apply_block(&devnet_genesis()).unwrap();
    // Build directly on genesis (mine_next would target height 1 correctly,
    // but here we construct the stale candidate manually for clarity).
    let tip_ts = chain.headers.last().unwrap().timestamp;
    let template = BlockTemplate {
        version: 1,
        previous_block: chain.tip.unwrap(),
        height: BlockHeight(1),
        timestamp: tip_ts + 10,
        target: u64::MAX,
        poarm_version: 0,
        poarm_nonce: 0,
    };
    let mk = |timestamp: u64, target: u64| {
        let mut b = template
            .build_mining_block(
                Vec::new(),
                miner.to_vec(),
                memobi_core::reward::RewardConfig {
                    initial_reward: params.initial_reward,
                    halving_interval: params.halving_interval,
                },
            )
            .unwrap();
        b.header.timestamp = timestamp;
        b.header.target = target;
        // Recompute root unchanged (no tx change); root stays valid.
        b.header.transaction_root = memobi_core::block::transaction_root(&b.transactions).unwrap();
        b
    };
    let stale = mk(tip_ts - 1, u64::MAX);
    assert_eq!(
        validate_block_header_with_params(
            &chain,
            &stale,
            Hash32([0u8; 32]),
            &params,
            tip_ts + 50,
            Some(tip_ts),
        ),
        Err(BlockValidationError::TimestampTooOld)
    );
    let future = mk(tip_ts + 10_000, u64::MAX);
    assert_eq!(
        validate_block_header_with_params(
            &chain,
            &future,
            Hash32([0u8; 32]),
            &params,
            tip_ts + 50,
            Some(tip_ts),
        ),
        Err(BlockValidationError::TimestampTooFar)
    );
    let bad = mk(tip_ts + 10, 0);
    assert_eq!(
        validate_block_header_with_params(
            &chain,
            &bad,
            Hash32([0u8; 32]),
            &params,
            tip_ts + 50,
            Some(tip_ts),
        ),
        Err(BlockValidationError::InvalidTarget)
    );
}

#[test]
fn heaviest_fork_is_preferred() {
    let easy = memobi_core::chainwork::cumulative_work([u64::MAX]);
    let hard = memobi_core::chainwork::cumulative_work([1]);
    assert!(memobi_core::chainwork::prefers_chain(hard, easy));
    let s = ChainState {
        work: easy,
        ..Default::default()
    };
    assert!(s.should_reorg(hard));
    assert!(!s.should_reorg(easy));
}

#[test]
fn wrong_difficulty_target_is_rejected() {
    use memobi_core::validation::{BlockValidationError, validate_block_header_with_params};
    let params = ConsensusParams::devnet();
    let miner = SecretKey::from_bytes(&[6u8; 32]).public_key();
    let mut chain = ChainState::default();
    chain.apply_block(&devnet_genesis()).unwrap();
    let tip_ts = chain.headers.last().unwrap().timestamp;
    let template = BlockTemplate {
        version: params.block_version,
        previous_block: chain.tip.unwrap(),
        height: BlockHeight(1),
        timestamp: tip_ts + 10,
        target: u64::MAX,
        poarm_version: params.poarm_version,
        poarm_nonce: 0,
    };
    let mut block = template
        .build_mining_block(
            Vec::new(),
            miner.to_vec(),
            memobi_core::reward::RewardConfig {
                initial_reward: params.initial_reward,
                halving_interval: params.halving_interval,
            },
        )
        .unwrap();
    // In-bounds target, but the tip (genesis) declares u64::MAX.
    block.header.target = u64::MAX / 2;
    assert_eq!(
        validate_block_header_with_params(
            &chain,
            &block,
            Hash32([0u8; 32]),
            &params,
            tip_ts + 50,
            Some(tip_ts),
        ),
        Err(BlockValidationError::InvalidDifficulty)
    );
}

#[test]
fn mempool_transactions_are_assembled_into_blocks() {
    let params = ConsensusParams::devnet();
    let miner_sk = SecretKey::from_bytes(&[4u8; 32]);
    let miner = miner_sk.public_key();
    let mut chain = ChainState::default();
    chain.apply_block(&devnet_genesis()).unwrap();
    // The same payout key is deliberately reused across blocks. Coinbase
    // height is committed by the reserved marker, so each reward has a
    // distinct transaction ID/outpoint and the height-1 reward can mature.
    let mut own = None;
    for height in 1u64..=11 {
        let tip_ts = chain.headers.last().unwrap().timestamp;
        let (b, p) = mine_next(&chain, &params, miner, tip_ts + 10, u64::MAX);
        if height == 1 {
            own = Some(b.clone());
        }
        chain.apply_validated_block(&b, p).unwrap();
    }
    let own = own.unwrap();
    assert_eq!(chain.height, Some(11));

    let op = memobi_core::OutPoint {
        txid: own.transactions[0].txid().unwrap(),
        index: 0,
    };
    let mut spend = memobi_core::Transaction {
        version: params.tx_version,
        inputs: vec![memobi_core::TxInput {
            previous_output: op,
            unlocking_data: Vec::new(),
        }],
        outputs: vec![memobi_core::TxOutput {
            value: params.initial_reward - 100,
            spending_condition: miner.to_vec(),
        }],
        fee: 100,
    };
    spend.inputs[0].unlocking_data = authorize_input(&spend, 0, &miner_sk, &params).unwrap();
    let spend_txid = spend.txid().unwrap();

    let mut mempool = memobi_core::mempool::Mempool::new();
    mempool
        .insert(
            spend.clone(),
            &chain.utxos,
            &params,
            chain.height.unwrap(),
            chain.headers.last().unwrap().timestamp + 10,
            0,
        )
        .unwrap();
    assert_eq!(mempool.len(), 1);

    // Assembly picks up the mempool candidate with correct linkage/difficulty.
    let tip_ts = chain.headers.last().unwrap().timestamp;
    let mut block = memobi_core::block_builder::build_block_from_mempool(
        &chain,
        &mempool,
        &params,
        tip_ts + 10,
        miner.to_vec(),
    )
    .unwrap();
    assert_eq!(block.header.height, BlockHeight(12));
    assert_eq!(block.header.previous_block, chain.tip.unwrap());
    assert_eq!(block.header.target, u64::MAX);
    assert_eq!(block.transactions.len(), 2);
    assert_eq!(block.transactions[1], spend);
    let coinbase_value: u64 = block.transactions[0].outputs.iter().map(|o| o.value).sum();
    let subsidy = memobi_core::reward::block_subsidy(
        44,
        memobi_core::reward::RewardConfig {
            initial_reward: params.initial_reward,
            halving_interval: params.halving_interval,
        },
    );
    assert_eq!(coinbase_value, subsidy + spend.fee);

    // Mine the proof and apply; the spend becomes confirmed state.
    let epoch = block.header.height.0;
    let seed = block.header.poarm_seed(epoch).unwrap();
    let found = memobi_poarm::miner::search_candidate_c(
        seed.as_bytes(),
        epoch,
        memobi_poarm::Config {
            memory_kib: 1,
            rounds: 1,
        },
        u64::MAX,
        0,
        100,
    )
    .unwrap();
    block.header.poarm_nonce = found.nonce;
    chain
        .apply_validated_block(&block, Hash32(found.proof))
        .unwrap();
    assert!(chain.utxos.contains_key(&memobi_core::OutPoint {
        txid: spend_txid,
        index: 0
    }));
    assert!(!chain.utxos.contains_key(&op));
}

#[test]
fn replay_fork_switches_to_heavier_chain() {
    let params = ConsensusParams::devnet();
    let main_pk = SecretKey::from_bytes(&[1u8; 32]).public_key();
    let fork_pk = SecretKey::from_bytes(&[2u8; 32]).public_key();

    // Main chain: genesis + 2 blocks.
    let mut main = ChainState::default();
    main.apply_block(&devnet_genesis()).unwrap();
    let mut main_blocks = vec![(devnet_genesis(), Hash32::ZERO)];
    for _ in 0..2 {
        let tip_ts = main.headers.last().unwrap().timestamp;
        let (b, p) = mine_next(&main, &params, main_pk, tip_ts + 10, u64::MAX);
        main.apply_validated_block(&b, p).unwrap();
        main_blocks.push((b, p));
    }
    assert_eq!(main.height, Some(2));

    // Fork from genesis: genesis + 4 blocks (diverges by miner identity).
    let mut fork = ChainState::default();
    fork.apply_block(&devnet_genesis()).unwrap();
    let mut fork_blocks = vec![(devnet_genesis(), Hash32::ZERO)];
    for _ in 0..4 {
        let tip_ts = fork.headers.last().unwrap().timestamp;
        let (b, p) = mine_next(&fork, &params, fork_pk, tip_ts + 20, u64::MAX);
        fork.apply_validated_block(&b, p).unwrap();
        fork_blocks.push((b, p));
    }

    // Longer fork wins the tie (devnet zero-work targets break ties by length).
    assert!(main.replay_fork(&fork_blocks).unwrap());
    assert_eq!(main.height, Some(4));
    assert_eq!(main.tip, fork.tip);
    assert_eq!(main.utxos, fork.utxos);

    // The shorter original chain cannot reorg back.
    assert!(!main.replay_fork(&main_blocks).unwrap());
    assert_eq!(main.height, Some(4));

    // A corrupted fork fails validation atomically: state unchanged.
    let mut broken = fork_blocks.clone();
    broken[1].0.header.previous_block = Hash32([9u8; 32]);
    assert!(main.replay_fork(&broken).is_err());
    assert_eq!(main.height, Some(4));
    assert_eq!(main.tip, fork.tip);
}
