//! Difficulty adjustment: deterministic retarget from chain timestamps.
//!
//! Every node computes the same target for identical history. Uses median
//! window sampling and bounded movement to resist timestamp manipulation.

use crate::params::ConsensusParams;

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
    pub fn from_params(params: &ConsensusParams) -> Self {
        Self {
            target_interval_secs: params.target_interval_secs,
            window: params.difficulty_window,
            max_target_change_numerator: params.max_target_change_numer,
            max_target_change_denominator: params.max_target_change_denom,
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

/// Compute the next target from header history.
///
/// - `timestamps`: oldest-first chain timestamps ending at the tip.
/// - `current_target`: tip target.
/// - Adjusts every `window` blocks; otherwise returns current target.
/// - Uses first/last of the window with per-block clamp to blunt
///   single-miner time-warp: each step capped at 2x interval.
pub fn next_target(
    timestamps: &[u64],
    current_target: u64,
    params: &ConsensusParams,
) -> Result<u64, DifficultyError> {
    if params.difficulty_window == 0 || params.target_interval_secs == 0 {
        return Err(DifficultyError::InvalidBounds);
    }
    if current_target < params.min_target || current_target > params.max_target {
        return Err(DifficultyError::InvalidBounds);
    }
    let window = params.difficulty_window as usize;
    if timestamps.len() < window + 1 {
        return Ok(current_target);
    }
    if !(timestamps.len() as u64 - 1).is_multiple_of(params.difficulty_window) {
        return Ok(current_target);
    }
    let start = timestamps.len() - window - 1;
    let slice = &timestamps[start..];
    // Clamp each step to [interval/2, interval*2] to limit time-warp gain.
    let mut clamped_elapsed: u64 = 0;
    for pair in slice.windows(2) {
        let mut step = pair[1].saturating_sub(pair[0]);
        let min_step = (params.target_interval_secs / 2).max(1);
        let max_step = params.target_interval_secs.saturating_mul(2).max(1);
        step = step.clamp(min_step, max_step);
        clamped_elapsed = clamped_elapsed
            .checked_add(step)
            .ok_or(DifficultyError::ArithmeticOverflow)?;
    }
    let obs = DifficultyObservation {
        first_timestamp: slice[0],
        last_timestamp: slice[0].saturating_add(clamped_elapsed),
        blocks: params.difficulty_window,
    };
    let res = adjust_target(
        current_target,
        obs,
        DifficultyConfig::from_params(params),
    )?;
    // Enforce absolute network bounds.
    Ok(res.new_target.clamp(params.min_target, params.max_target))
}

/// Target a block extending `history` (oldest-first, ending at the tip) with
/// `candidate_timestamp` must declare.
///
/// - Empty `history` => candidate is genesis: returns `params.genesis_target`.
/// - Otherwise `tip_target` must equal the tip header's target, and the
///   retarget boundary rule is delegated to [`next_target`] on the extended
///   history, so validation and mining derive identical targets.
pub fn next_block_target(
    history: &[u64],
    tip_target: Option<u64>,
    candidate_timestamp: u64,
    params: &ConsensusParams,
) -> Result<u64, DifficultyError> {
    if history.is_empty() {
        return Ok(params.genesis_target);
    }
    let tip_target = tip_target.ok_or(DifficultyError::InvalidBounds)?;
    let mut extended = Vec::with_capacity(history.len() + 1);
    extended.extend_from_slice(history);
    extended.push(candidate_timestamp);
    next_target(&extended, tip_target, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_block_target_pins_genesis_and_carries_until_boundary() {
        let params = ConsensusParams::devnet();
        // Genesis: no history -> pinned target.
        assert_eq!(
            next_block_target(&[], None, params.genesis_timestamp, &params).unwrap(),
            params.genesis_target
        );
        // Below the retarget boundary: carry the tip target forward.
        let history: Vec<u64> = (0u64..59).map(|i| i * params.target_interval_secs).collect();
        assert_eq!(
            next_block_target(&history, Some(u64::MAX), 1_000, &params).unwrap(),
            u64::MAX
        );
        // Window boundary (60 historical + candidate = 61): retarget using
        // per-step clamped elapsed; 5s steps (clamped min) halve the target.
        let fast: Vec<u64> = (0u64..60).map(|i| i * (params.target_interval_secs / 2)).collect();
        let candidate_ts = *fast.last().unwrap() + params.target_interval_secs / 2;
        assert_eq!(
            next_block_target(&fast, Some(u64::MAX), candidate_ts, &params).unwrap(),
            u64::MAX / 2
        );
        // History without a tip target cannot be extended.
        assert_eq!(
            next_block_target(&history, None, 1_000, &params),
            Err(DifficultyError::InvalidBounds)
        );
    }

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

    #[test]
    fn retarget_is_stable_and_bounded() {
        let mut params = ConsensusParams::devnet();
        params.difficulty_window = 4;
        params.target_interval_secs = 10;
        params.min_target = 1;
        params.max_target = u64::MAX;
        // Not at boundary: no change (3 intervals, window 4).
        let ts = vec![0, 10, 20, 30];
        assert_eq!(next_target(&ts, 1000, &params).unwrap(), 1000);
        // Perfect window (4 intervals): unchanged.
        let ts = vec![0, 10, 20, 30, 40];
        assert_eq!(next_target(&ts, 1000, &params).unwrap(), 1000);
    }

    #[test]
    fn retarget_resists_time_warp() {
        let mut params = ConsensusParams::devnet();
        params.difficulty_window = 4;
        params.target_interval_secs = 10;
        params.min_target = 1;
        params.max_target = u64::MAX;
        // Attacker claims instant blocks; per-step clamp limits easing.
        let ts = vec![0, 0, 0, 0, 0];
        let next = next_target(&ts, 1000, &params).unwrap();
        // Clamped steps: each 0 saturates to min 5s => observed 5s vs 10s
        // => unclamped 500, within 4x bound.
        assert_eq!(next, 500);
    }

    #[test]
    fn retarget_clamps_to_network_bounds() {
        let mut params = ConsensusParams::devnet();
        params.difficulty_window = 2;
        params.target_interval_secs = 10;
        params.min_target = 900;
        params.max_target = 1100;
        // 2 intervals of 1000s each, clamped to 20s => observed 20s vs 10s
        // => unclamped 2000, then max_target 1100.
        let ts = vec![0, 1000, 2000];
        let next = next_target(&ts, 1000, &params).unwrap();
        assert_eq!(next, 1100);
    }
}
