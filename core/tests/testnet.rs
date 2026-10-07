//! Short Testnet smoke coverage.
//!
//! Testnet is intentionally small here: it proves the network identity and
//! real consensus path before public infrastructure is introduced.

use memobi_core::{\n    genesis::testnet_genesis,\n    hash::Hash32,\n    node::Node,\n    params::ConsensusParams,\n    p2p::Message,\n    sync::SyncState,\n};
use memobi_poarm::devnet::{DevnetProducerConfig, produce_for_params};

#[test]
fn testnet_has_distinct_genesis_and_real_blocks() {
    let params = ConsensusParams::testnet();
    let result = produce_for_params(
        params,
        DevnetProducerConfig {
            blocks: 8,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.chain.height, Some(8));
    assert_eq!(
        result.chain.headers[0].block_id().unwrap(),
        testnet_genesis().header.block_id().unwrap()
    );
    assert_ne!(
        result.chain.headers[0].block_id().unwrap(),
        memobi_core::genesis::devnet_genesis()
            .header
            .block_id()
            .unwrap()
    );

    for (index, produced) in result.blocks.iter().enumerate() {
        assert_eq!(produced.block.header.height.0, (index + 1) as u64);
    }
}

#[test]
fn testnet_two_node_header_then_block_sync_converges() {
    let params = ConsensusParams::testnet();
    let produced = produce_for_params(
        params.clone(),
        DevnetProducerConfig {
            blocks: 4,
            ..Default::default()
        },
    )
    .unwrap();

    let mut source = Node::new(params.clone());
    let genesis = testnet_genesis();
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
            let dir = std::env::temp_dir().join(format!(
                "memobi-testnet-source-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("blocks");
            let mut store = memobi_core::storage::BlockStore::open(&path).unwrap();
            store.append(&genesis, Hash32::ZERO).unwrap();
            for entry in &produced.blocks {
                store.append(&entry.block, entry.proof).unwrap();
            }
            let response = source.serve_get_blocks(start_height, count, &mut store).unwrap();
            drop(store);
            let _ = std::fs::remove_dir_all(dir);
            response
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
                Message::Blocks { blocks: vec![envelope] },
                block.header.timestamp,
                None,
            )
            .unwrap();
    }

    assert_eq!(target.chain.height, Some(4));
    assert_eq!(target.chain.tip, source.chain.tip);
    assert_eq!(target.chain.utxos, source.chain.utxos);
    assert_eq!(target.sync.progress.state, SyncState::Synced);
}

