//! Consensus parameters — single source of truth for protocol limits.
//!
//! No consensus value may be hard-coded elsewhere; all validation,
//! construction and tests must read from [`ConsensusParams`].

use crate::COIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsensusParams {
    /// Human-readable chain identifier (e.g. `memobi-devnet-0`).
    pub network_id: &'static str,
    /// Numeric chain identifier for replay protection / sighash domain.
    pub chain_id: u32,
    /// Address human-readable part (e.g. `mb`, `mbt`, `mbd`).
    pub address_hrp: &'static str,
    /// Accepted transaction versions.
    pub tx_version: u32,
    /// Accepted block versions.
    pub block_version: u32,
    /// Maximum serialized transaction size in bytes.
    pub max_tx_bytes: usize,
    /// Maximum inputs / outputs per transaction.
    pub max_inputs_per_tx: usize,
    pub max_outputs_per_tx: usize,
    /// Maximum bytes for unlocking data / spending condition per item.
    pub max_unlocking_bytes: usize,
    pub max_spending_condition_bytes: usize,
    /// Maximum transactions per block (including coinbase).
    pub max_txs_per_block: usize,
    /// Maximum serialized block size in bytes (header + txs).
    pub max_block_bytes: usize,
    /// Maximum money supply in base units (satoshis-equivalent).
    pub max_money: u64,
    /// Maximum block subsidy output value (sanity cap).
    pub max_block_subsidy: u64,
    /// Coinbase maturity: blocks before coinbase UTXO is spendable.
    pub coinbase_maturity: u64,
    /// Target block interval (seconds).
    pub target_interval_secs: u64,
    /// Difficulty adjustment window (blocks).
    pub difficulty_window: u64,
    /// Maximum target movement numerator / denominator (e.g. 4/1 = 4x).
    pub max_target_change_numer: u64,
    pub max_target_change_denom: u64,
    /// Minimum allowed target (hardest) and maximum (easiest).
    pub min_target: u64,
    pub max_target: u64,
    /// Timestamp rules: max seconds into the future vs median-past.
    pub max_future_skew_secs: u64,
    /// PoARM version accepted.
    pub poarm_version: u32,
    /// PoARM domain separation tag for header seed.
    pub poarm_domain: &'static str,
    /// Genesis timestamp and initial target.
    pub genesis_timestamp: u64,
    pub genesis_target: u64,
    /// Block reward schedule.
    pub initial_reward: u64,
    pub halving_interval: u64,
    /// P2P limits.
    pub p2p_max_collection_items: usize,
    pub p2p_max_payload_bytes: usize,
    pub mempool_max_txs: usize,
    pub mempool_max_bytes: usize,
    pub mempool_max_age_secs: u64,
}

impl ConsensusParams {
    pub const fn devnet() -> Self {
        Self {
            network_id: "memobi-devnet-0",
            chain_id: 0x6D62_3030, // "mb00" le
            address_hrp: "mbd",
            tx_version: 1,
            block_version: 1,
            max_tx_bytes: 100_000,
            max_inputs_per_tx: 256,
            max_outputs_per_tx: 256,
            max_unlocking_bytes: 16_384,
            max_spending_condition_bytes: 8_192,
            max_txs_per_block: 2_048,
            max_block_bytes: 2 * 1024 * 1024,
            max_money: 100_000_000 * COIN,
            max_block_subsidy: 50 * COIN,
            coinbase_maturity: 10,
            target_interval_secs: 10,
            difficulty_window: 60,
            max_target_change_numer: 4,
            max_target_change_denom: 1,
            min_target: 1,
            max_target: u64::MAX,
            max_future_skew_secs: 120,
            poarm_version: 0,
            poarm_domain: "MEMOBI-POARM-V0",
            genesis_timestamp: 1_700_000_000,
            genesis_target: u64::MAX,
            initial_reward: 50 * COIN,
            halving_interval: 210_000,
            p2p_max_collection_items: 4_096,
            p2p_max_payload_bytes: 2 * 1024 * 1024,
            mempool_max_txs: 10_000,
            mempool_max_bytes: 10 * 1024 * 1024,
            mempool_max_age_secs: 72 * 3600,
        }
    }

    pub const fn testnet() -> Self {
        Self {
            network_id: "memobi-testnet-0",
            chain_id: 0x6D62_7430,
            address_hrp: "mbt",
            ..Self::devnet()
        }
    }

    pub const fn mainnet_candidate() -> Self {
        Self {
            network_id: "memobi-mainnet-0",
            chain_id: 0x6D62_3130,
            address_hrp: "mb",
            ..Self::devnet()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn devnet_and_testnet_ids_differ() {
        assert_ne!(ConsensusParams::devnet().chain_id, ConsensusParams::testnet().chain_id);
        assert_ne!(ConsensusParams::devnet().network_id, ConsensusParams::testnet().network_id);
        assert_ne!(ConsensusParams::devnet().address_hrp, ConsensusParams::testnet().address_hrp);
    }
    #[test]
    fn money_bounds_are_sane() {
        let p = ConsensusParams::devnet();
        assert!(p.max_money <= 21_000_000 * 100_000_000 * 1000);
        assert!(p.max_block_subsidy <= p.max_money);
        assert!(p.min_target >= 1);
        assert!(p.max_target >= p.min_target);
    }
}
