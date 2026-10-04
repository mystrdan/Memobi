//! Experimental difficulty-adjustment simulator.
//!
//! This module is laboratory code only. Its output is not consensus-critical
//! until timestamp rules, integer rounding, bounds, and window semantics are
//! formally specified and frozen.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifficultyConfig {
    pub target_interval_secs: u64,
    pub window: u64,
    pub max_target_change_numerator: u64,
    pub max_target_change_denominator: u64,
}

impl DifficultyConfig {
    pub fn baseline() -> Self {
        Self {
            target_interval_secs: 10,
            window: 720,
            max_target_change_numerator: 4,
            max_target_change_denominator: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifficultyObservation {
    pub first_timestamp: u64,
    pub last_timestamp: u64,
    pub blocks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifficultyResult {
    pub old_target: u64,
    pub observed_interval_secs: u64,
    pub unclamped_target: u64,
    pub new_target: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifficultyError {
    ZeroBlocks,
    TimestampUnderflow,
    InvalidBounds,
    ArithmeticOverflow,
}

/// Baseline laboratory rule:
/// new_target = old_target * observed_interval / target_interval,
/// followed by a bounded target movement.
///
/// Larger target means easier PoW in this simulator.
pub fn adjust_target(
    old_target: u64,
    observation: DifficultyObservation,
    config: DifficultyConfig,
) -> Result<DifficultyResult, DifficultyError> {
    if observation.blocks == 0 {
        return Err(DifficultyError::ZeroBlocks);
    }
    if observation.last_timestamp < observation.first_timestamp {
        return Err(DifficultyError::TimestampUnderflow);
    }
    if config.target_interval_secs == 0
        || config.max_target_change_denominator == 0
        || config.max_target_change_numerator == 0
    {
        return Err(DifficultyError::InvalidBounds);
    }

    let elapsed = observation.last_timestamp - observation.first_timestamp;
    let observed_interval = elapsed / observation.blocks;

    let unclamped = u128::from(old_target)
        .checked_mul(u128::from(observed_interval))
        .and_then(|v| v.checked_div(u128::from(config.target_interval_secs)))
        .ok_or(DifficultyError::ArithmeticOverflow)?;

    let max_up = u128::from(old_target)
        .checked_mul(u128::from(config.max_target_change_numerator))
        .and_then(|v| v.checked_div(u128::from(config.max_target_change_denominator)))
        .ok_or(DifficultyError::ArithmeticOverflow)?;

    let min_target = u128::from(old_target)
        .checked_mul(u128::from(config.max_target_change_denominator))
        .and_then(|v| {
            let n = u128::from(config.max_target_change_numerator);
            v.checked_div(n)
        })
        .ok_or(DifficultyError::ArithmeticOverflow)?;

    let max_target = max_up.min(u128::from(u64::MAX));
    let bounded = unclamped.clamp(min_target, max_target);
    let new_target = u64::try_from(bounded).map_err(|_| DifficultyError::ArithmeticOverflow)?;

    Ok(DifficultyResult {
        old_target,
        observed_interval_secs: observed_interval,
        unclamped_target: u64::try_from(unclamped.min(u128::from(u64::MAX)))
            .map_err(|_| DifficultyError::ArithmeticOverflow)?,
        new_target,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticMiner {
    pub blocks_per_window: u64,
}

/// Deterministic timestamp generator for a stable synthetic population.
pub fn stable_observation(
    first_timestamp: u64,
    config: DifficultyConfig,
    miner_speed_multiplier: u64,
) -> DifficultyObservation {
    let multiplier = miner_speed_multiplier.max(1);
    let interval = (config.target_interval_secs / multiplier).max(1);
    DifficultyObservation {
        first_timestamp,
        last_timestamp: first_timestamp + interval * config.window,
        blocks: config.window,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_window_preserves_target() {
        let config = DifficultyConfig::baseline();
        let observation = DifficultyObservation {
            first_timestamp: 1_000,
            last_timestamp: 1_000 + config.target_interval_secs * config.window,
            blocks: config.window,
        };
        let result = adjust_target(1_000_000, observation, config).unwrap();
        assert_eq!(result.observed_interval_secs, 10);
        assert_eq!(result.new_target, 1_000_000);
    }

    #[test]
    fn faster_blocks_make_target_harder() {
        let config = DifficultyConfig::baseline();
        let observation = DifficultyObservation {
            first_timestamp: 0,
            last_timestamp: 5 * config.window,
            blocks: config.window,
        };
        let result = adjust_target(1_000_000, observation, config).unwrap();
        assert_eq!(result.new_target, 500_000);
    }

    #[test]
    fn slower_blocks_are_bounded() {
        let config = DifficultyConfig::baseline();
        let observation = DifficultyObservation {
            first_timestamp: 0,
            last_timestamp: 100 * config.window,
            blocks: config.window,
        };
        let result = adjust_target(1_000_000, observation, config).unwrap();
        assert_eq!(result.new_target, 4_000_000);
    }

    #[test]
    fn invalid_window_is_rejected() {
        let config = DifficultyConfig::baseline();
        let observation = DifficultyObservation {
            first_timestamp: 0,
            last_timestamp: 10,
            blocks: 0,
        };
        assert_eq!(
            adjust_target(1_000, observation, config),
            Err(DifficultyError::ZeroBlocks)
        );
    }
}
