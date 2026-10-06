//! Real two-node synchronization and restart recovery using the existing consensus path.
use memobi_core::{
    Hash32,
    node::Node,
    p2p::Message,
    params::ConsensusParams,
    storage::{BlockStore, HeaderStore},
    sync::SyncState,
};
use memobi_poarm::devnet::{DevnetProducerConfig, produce};

#[test]
fn two_nodes_converge_on_real_blocks() {
    let params = ConsensusParams::devnet();
    let produced = produce(DevnetProducerConfig {
        blocks: 4,
        ..Default::default()
    })
    .unwrap();
    let mut source = Node::new(params.clone());
    let mut target = Node::new(params.clone());
    let genesis = memobi_core::genesis::devnet_genesis();
    source
        .apply_received_block(
            genesis.clone(),
            Hash32::ZERO,
            genesis.header.timestamp,
            None,
        )
        .unwrap();
    for entry in &produced.blocks {
        source
            .apply_received_block(
                entry.block.clone(),
                entry.proof,
                entry.block.header.timestamp,
                None,
            )
            .unwrap();
    }
    let source_version = source.start_peer();
    target
        .receive_peer_message(source_version, 0, None)
        .unwrap();
    let target_version = target.start_peer();
    source
        .receive_peer_message(target_version, 0, None)
        .unwrap();
    assert_eq!(target.peer.remote_height, Some(4));
    let envelopes = {
        let mut v = vec![Message::encode_block(&genesis, Hash32::ZERO).unwrap()];
        v.extend(
            produced
                .blocks
                .iter()
                .map(|b| Message::encode_block(&b.block, b.proof).unwrap()),
        );
        v
    };
    for envelope in envelopes {
        let (block, _) = Message::decode_block(&envelope, &params).unwrap();
        target
            .receive_peer_message(
                Message::Blocks {
                    blocks: vec![envelope],
                },
                block.header.timestamp,
                None,
            )
            .unwrap();
    }
    assert_eq!(target.chain.height, Some(4));
    assert_eq!(target.chain.tip, source.chain.tip);
    assert_eq!(target.chain.utxos, produced.chain.utxos);
    assert_eq!(target.sync.progress.state, SyncState::Synced);
}


#[test]
fn nodes_sync_headers_then_blocks_through_serving_path() {
    let params = ConsensusParams::devnet();
    let produced = produce(DevnetProducerConfig {
        blocks: 4,
        ..Default::default()
    })
    .unwrap();

    let dir = std::env::temp_dir().join(format!(
        "memobi-sync-serving-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let block_path = dir.join("source.blocks");
    let header_path = dir.join("source.headers");
    let mut source_blocks = BlockStore::open(&block_path).unwrap();
    let mut source_headers = HeaderStore::open(&header_path).unwrap();

    let genesis = memobi_core::genesis::devnet_genesis();
    let mut source = Node::new(params.clone());
    source
        .apply_received_block_with_stores(
            genesis.clone(),
            Hash32::ZERO,
            genesis.header.timestamp,
            &mut source_headers,
            &mut source_blocks,
        )
        .unwrap();
    for entry in &produced.blocks {
        source
            .apply_received_block_with_stores(
                entry.block.clone(),
                entry.proof,
                entry.block.header.timestamp,
                &mut source_headers,
                &mut source_blocks,
            )
            .unwrap();
    }

    let mut target = Node::new(params.clone());
    let source_version = source.start_peer();
    target.receive_peer_message(source_version, 0, None).unwrap();
    let target_version = target.start_peer();
    source.receive_peer_message(target_version, 0, None).unwrap();

    let header_request = target.next_sync_message().unwrap();
    let header_response = match header_request {
        Message::GetHeaders { locator } => source.serve_get_headers(&locator).unwrap(),
        other => panic!("expected GetHeaders, got {other:?}"),
    };
    target
        .receive_peer_message(header_response, genesis.header.timestamp, None)
        .unwrap();

    let block_request = target.next_sync_message().unwrap();
    let block_response = match block_request {
        Message::GetBlocks { start_height, count } => {
            source.serve_get_blocks(start_height, count, &mut source_blocks).unwrap()
        }
        other => panic!("expected GetBlocks, got {other:?}"),
    };
    let blocks = match block_response {
        Message::Blocks { blocks } => blocks,
        other => panic!("expected Blocks, got {other:?}"),
    };
    for envelope in blocks {
        let (block, _) = Message::decode_block(&envelope, &params).unwrap();
        target
            .receive_peer_message(
                Message::Blocks {
                    blocks: vec![envelope],
                },
                block.header.timestamp,
                None,
            )
            .unwrap();
    }

    assert_eq!(target.chain.height, source.chain.height);
    assert_eq!(target.chain.tip, source.chain.tip);
    assert_eq!(target.chain.utxos, source.chain.utxos);
    assert_eq!(target.sync.progress.state, SyncState::Synced);

    drop(source_blocks);
    drop(source_headers);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn durable_blocks_rebuild_consensus_state_after_restart() {
    let params = ConsensusParams::devnet();
    let produced = produce(DevnetProducerConfig {
        blocks: 4,
        ..Default::default()
    })
    .unwrap();
    let dir = std::env::temp_dir().join(format!("memobi-recovery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let block_path = dir.join("canonical.blocks");
    let header_path = dir.join("canonical.headers");
    let mut blocks = BlockStore::open(&block_path).unwrap();
    let mut headers = HeaderStore::open(&header_path).unwrap();
    let genesis = memobi_core::genesis::devnet_genesis();
    let mut chain = memobi_core::chain::ChainState::default();
    chain
        .apply_validated_block_with_stores(
            &genesis,
            Hash32::ZERO,
            &params,
            genesis.header.timestamp,
            None,
            &mut headers,
            &mut blocks,
        )
        .unwrap();
    for entry in &produced.blocks {
        let parent = chain.headers.last().map(|h| h.timestamp);
        chain
            .apply_validated_block_with_stores(
                &entry.block,
                entry.proof,
                &params,
                entry.block.header.timestamp,
                parent,
                &mut headers,
                &mut blocks,
            )
            .unwrap();
    }
    drop(blocks);
    drop(headers);
    let mut reopened = BlockStore::open(&block_path).unwrap();
    let recovered = Node::recover_from_block_store(params, &mut reopened).unwrap();
    assert_eq!(recovered.chain.height, produced.chain.height);
    assert_eq!(recovered.chain.tip, produced.chain.tip);
    assert_eq!(recovered.chain.utxos, produced.chain.utxos);
    let _ = std::fs::remove_dir_all(dir);
}
