//! Short Testnet smoke coverage.
//!
//! Testnet is intentionally small here: it proves the network identity and
//! real consensus path before public infrastructure is introduced.

use memobi_core::{genesis::testnet_genesis, params::ConsensusParams};
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
