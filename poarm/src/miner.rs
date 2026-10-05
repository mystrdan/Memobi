//! PoARM mining and independent verification (devnet candidate C).
//!
//! Search and verify share one rule: `work_candidate_c(seed,nonce,epoch)`
//! recomputed from the header seed, first 8 bytes BE compared to target.
//! Verification is independent: it needs only header + params, never miner state.

use crate::{Config, work_candidate_c};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MiningResult {
    pub nonce: u64,
    pub proof: [u8; 32],
    pub attempts: u64,
}

/// Search sequential nonces until Candidate C meets target.
pub fn search_candidate_c(
    seed: &[u8],
    epoch: u64,
    config: Config,
    target: u64,
    start_nonce: u64,
    max_attempts: u64,
) -> Option<MiningResult> {
    if config.memory_kib == 0 || config.memory_kib > 1_048_576 || config.rounds == 0 || config.rounds > 64 {
        return None;
    }
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

/// Independently verify a `(seed, nonce, epoch, proof)` tuple.
/// Recomputes the workload; rejects mismatched proofs and above-target proofs.
pub fn verify_candidate_c(
    seed: &[u8],
    epoch: u64,
    config: Config,
    target: u64,
    nonce: u64,
    proof: &[u8; 32],
) -> bool {
    if config.memory_kib == 0 || config.memory_kib > 1_048_576 || config.rounds == 0 || config.rounds > 64 {
        return false;
    }
    let expected = work_candidate_c(seed, nonce, epoch, config);
    if &expected != proof {
        return false;
    }
    u64::from_be_bytes(proof[..8].try_into().unwrap()) <= target
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

    #[test]
    fn verify_recomputes_and_checks_target() {
        use super::verify_candidate_c;
        let config = Config {
            memory_kib: 1,
            rounds: 1,
        };
        let found = search_candidate_c(b"memobi", 0, config, u64::MAX, 0, 5).unwrap();
        assert!(verify_candidate_c(
            b"memobi",
            0,
            config,
            u64::MAX,
            found.nonce,
            &found.proof
        ));
        // Wrong proof bytes rejected.
        let mut bad = found.proof;
        bad[0] ^= 0xff;
        assert!(!verify_candidate_c(b"memobi", 0, config, u64::MAX, found.nonce, &bad));
        // Above-target rejected (target 0 unless proof is zero).
        assert!(!verify_candidate_c(b"memobi", 0, config, 0, found.nonce, &found.proof)
            || found.proof[..8] == [0u8; 8]);
    }

    #[test]
    fn invalid_config_is_rejected() {
        use super::verify_candidate_c;
        let bad = Config {
            memory_kib: 0,
            rounds: 1,
        };
        assert!(search_candidate_c(b"x", 0, bad, u64::MAX, 0, 10).is_none());
        assert!(!verify_candidate_c(b"x", 0, bad, u64::MAX, 0, &[0u8; 32]));
    }
}
