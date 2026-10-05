//! Real two-node synchronization and restart recovery using the existing consensus path.
use memobi_core::{Hash32, node::Node, params::ConsensusParams, p2p::Message, storage::{BlockStore, HeaderStore}, sync::SyncState};
use memobi_poarm::devnet::{produce, DevnetProducerConfig};

#[test]
fn two_nodes_converge_on_real_blocks() {
    let params = ConsensusParams::devnet();
    let produced = produce(DevnetProducerConfig { blocks: 4, ..Default::default() }).unwrap();
    let mut source = Node::new(params.clone());
    let mut target = Node::new(params.clone());
    let genesis = memobi_core::genesis::devnet_genesis();
    source.apply_received_block(genesis.clone(), Hash32::ZERO, genesis.header.timestamp, None).unwrap();
    for entry in &produced.blocks {
        source.apply_received_block(entry.block.clone(), entry.proof, entry.block.header.timestamp, None).unwrap();
    }
    let source_version = source.start_peer();
    target.receive_peer_message(source_version, 0, None).unwrap();
    let target_version = target.start_peer();
    source.receive_peer_message(target_version, 0, None).unwrap();
    assert_eq!(target.peer.remote_height, Some(4));
    let envelopes = {
        let mut v = vec![Message::encode_block(&genesis, Hash32::ZERO).unwrap()];
        v.extend(produced.blocks.iter().map(|b| Message::encode_block(&b.block, b.proof).unwrap()));
        v
    };
    for envelope in envelopes {
        let (block, _) = Message::decode_block(&envelope, &params).unwrap();
        target.receive_peer_message(Message::Blocks { blocks: vec![envelope] }, block.header.timestamp, None).unwrap();
    }
    assert_eq!(target.chain.height, Some(4));
    assert_eq!(target.chain.tip, source.chain.tip);
    assert_eq!(target.chain.utxos, produced.chain.utxos);
    assert_eq!(target.sync.progress.state, SyncState::Synced);
}

#[test]
fn durable_blocks_rebuild_consensus_state_after_restart() {
    let params = ConsensusParams::devnet();
    let produced = produce(DevnetProducerConfig { blocks: 4, ..Default::default() }).unwrap();
    let dir = std::env::temp_dir().join(format!("memobi-recovery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let block_path = dir.join("canonical.blocks");
    let header_path = dir.join("canonical.headers");
    let mut blocks = BlockStore::open(&block_path).unwrap();
    let mut headers = HeaderStore::open(&header_path).unwrap();
    let genesis = memobi_core::genesis::devnet_genesis();
    let mut chain = memobi_core::chain::ChainState::default();
    chain.apply_validated_block_with_stores(&genesis, Hash32::ZERO, &params, genesis.header.timestamp, None, &mut headers, &mut blocks).unwrap();
    for entry in &produced.blocks {
        let parent = chain.headers.last().map(|h| h.timestamp);
        chain.apply_validated_block_with_stores(&entry.block, entry.proof, &params, entry.block.header.timestamp, parent, &mut headers, &mut blocks).unwrap();
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