//! Chain selection by cumulative proof-of-work.
//!
//! Smaller targets mean harder work. Score is monotonic and deterministic:
//! `MAX/(target+1)` per block, summed with saturating arithmetic.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct WorkScore(pub u128);

/// Approximate proof-of-work contribution for a u64 target.
pub fn block_work(target: u64) -> WorkScore {
    WorkScore(u128::from(u64::MAX) / (u128::from(target) + 1))
}

pub fn cumulative_work<I>(targets: I) -> WorkScore
where
    I: IntoIterator<Item = u64>,
{
    WorkScore(
        targets
            .into_iter()
            .map(|target| block_work(target).0)
            .fold(0u128, |a, b| a.saturating_add(b)),
    )
}

pub fn prefers_chain(candidate: WorkScore, current: WorkScore) -> bool {
    candidate.0 > current.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harder_target_has_more_work() {
        assert!(block_work(100).0 > block_work(1_000).0);
    }

    #[test]
    fn cumulative_work_is_additive() {
        let score = cumulative_work([100, 1_000]);
        assert_eq!(score.0, block_work(100).0 + block_work(1_000).0);
    }

    #[test]
    fn higher_work_chain_wins() {
        assert!(prefers_chain(block_work(100), block_work(1_000)));
        assert!(!prefers_chain(block_work(1_000), block_work(100)));
    }
}
