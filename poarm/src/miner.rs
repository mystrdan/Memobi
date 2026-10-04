//! Experimental PoARM mining loop.
//!
//! This is laboratory code. Its target interpretation intentionally mirrors
//! the temporary core PoW helper and must not be treated as final consensus.

use crate::{Config, work_candidate_c};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiningResult {
    pub nonce: u64,
    pub proof: [u8; 32],
    pub attempts: u64,
}

/// Search sequential nonces until Candidate C produces a proof whose first
/// eight bytes, interpreted as big-endian, are at or below target.
///
/// max_attempts bounds the laboratory search so callers can safely use this
/// helper in benchmarks and tests.
pub fn search_candidate_c(
    seed: &[u8],
    epoch: u64,
    config: Config,
    target: u64,
    start_nonce: u64,
    max_attempts: u64,
) -> Option<MiningResult> {
    for offset in 0..max_attempts {
        let nonce = start_nonce.checked_add(offset)?;
        let proof = work_candidate_c(seed, nonce, epoch, config);
        let value = u64::from_be_bytes(proof[..8].try_into().unwrap());

        if value <= target {
            return Some(MiningResult {
                nonce,
                proof,
                attempts: offset + 1,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_easy_target() {
        let config = Config {
            memory_kib: 1,
            rounds: 1,
        };

        let result = search_candidate_c(b"memobi", 0, config, u64::MAX, 0, 1).unwrap();

        assert_eq!(result.nonce, 0);
        assert_eq!(result.attempts, 1);
    }

    #[test]
    fn search_respects_attempt_limit() {
        let config = Config {
            memory_kib: 1,
            rounds: 1,
        };

        assert!(search_candidate_c(b"memobi", 0, config, 0, 0, 0).is_none());
    }
}
