//! Provisional block-reward helpers.
//!
//! Economics are not consensus-frozen. This module provides a deterministic
//! laboratory model so emission scenarios can be measured before finalization.

use crate::COIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewardConfig {
    pub initial_reward: u64,
    pub halving_interval: u64,
}

impl RewardConfig {
    pub const fn provisional() -> Self {
        Self {
            initial_reward: 50 * COIN,
            halving_interval: 210_000,
        }
    }
}

/// Return the subsidy for a block height under the provisional halving model.
///
/// The result becomes zero after 63 halvings, avoiding undefined behavior and
/// making the emission tail deterministic for simulations.
pub fn block_subsidy(height: u64, config: RewardConfig) -> u64 {
    if config.halving_interval == 0 {
        return 0;
    }

    let halvings = height / config.halving_interval;
    if halvings >= 63 {
        return 0;
    }

    config.initial_reward >> halvings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsidy_starts_at_initial_reward() {
        let config = RewardConfig::provisional();
        assert_eq!(block_subsidy(0, config), 50 * COIN);
    }

    #[test]
    fn subsidy_halves_at_interval() {
        let config = RewardConfig {
            initial_reward: 100,
            halving_interval: 10,
        };
        assert_eq!(block_subsidy(9, config), 100);
        assert_eq!(block_subsidy(10, config), 50);
        assert_eq!(block_subsidy(20, config), 25);
    }

    #[test]
    fn zero_interval_disables_reward_model() {
        let config = RewardConfig {
            initial_reward: 100,
            halving_interval: 0,
        };
        assert_eq!(block_subsidy(0, config), 0);
    }
}
