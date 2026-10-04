//! Experimental PoARM laboratory implementation.
//!
//! This is research code and is NOT consensus-ready.
//! The workload exists to study memory pressure, sequential dependencies,
//! scaling, and cross-platform determinism.

use sha2::{Digest, Sha256};

pub const VERSION: u32 = 0;
pub const DEFAULT_MEMORY_KIB: usize = 1024;
pub const DEFAULT_ROUNDS: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub memory_kib: usize,
    pub rounds: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            memory_kib: DEFAULT_MEMORY_KIB,
            rounds: DEFAULT_ROUNDS,
        }
    }
}

fn seed_digest(seed: &[u8], nonce: u64) -> [u8; 32] {
    let mut input = Vec::with_capacity(seed.len() + 8);
    input.extend_from_slice(seed);
    input.extend_from_slice(&nonce.to_le_bytes());
    Sha256::digest(input).into()
}

/// Candidate A / PoARM v0 baseline.
pub fn work(seed: &[u8], nonce: u64, config: Config) -> [u8; 32] {
    let words = (config.memory_kib.max(1) * 1024) / 8;
    let mut memory = vec![0u64; words];
    let digest = seed_digest(seed, nonce);

    for (i, slot) in memory.iter_mut().enumerate() {
        let offset = (i * 8) % digest.len();
        let mut chunk = [0u8; 8];
        for j in 0..8 {
            chunk[j] = digest[(offset + j) % digest.len()];
        }
        *slot = u64::from_le_bytes(chunk) ^ (i as u64).rotate_left((i % 63) as u32);
    }

    let mut state = u64::from_le_bytes(digest[0..8].try_into().unwrap());

    for round in 0..config.rounds {
        for i in 0..memory.len() {
            let index = ((state as usize) ^ i) % memory.len();
            let value = memory[index];
            state = state
                .wrapping_add(value)
                .rotate_left(((i + round as usize) % 63 + 1) as u32)
                ^ (i as u64);
            memory[i] = memory[i].wrapping_add(state ^ value);
        }
    }

    finalize(seed, nonce, state, &memory)
}

/// Candidate B: a tighter state-dependent chain.
///
/// Unlike the baseline's index derivation, each step's next memory position is
/// derived primarily from the previous state. This is an experiment in making
/// the dependency chain harder to split into independent lanes.
pub fn work_candidate_b(seed: &[u8], nonce: u64, config: Config) -> [u8; 32] {
    let words = (config.memory_kib.max(1) * 1024) / 8;
    let mut memory = vec![0u64; words];
    let digest = seed_digest(seed, nonce);

    for (i, slot) in memory.iter_mut().enumerate() {
        let mut h = Sha256::new();
        h.update(digest);
        h.update((i as u64).to_le_bytes());
        *slot = u64::from_le_bytes(h.finalize()[0..8].try_into().unwrap());
    }

    let mut state = u64::from_le_bytes(digest[0..8].try_into().unwrap());
    for round in 0..config.rounds {
        for step in 0..memory.len() {
            let index = (state as usize) % memory.len();
            let value = memory[index];
            state = state
                .wrapping_add(value ^ (step as u64))
                .rotate_left(((round as usize + step) % 63 + 1) as u32);
            memory[index] = value
                .wrapping_add(state)
                .rotate_right(((state as usize) % 63 + 1) as u32);
        }
    }

    finalize(seed, nonce, state, &memory)
}

/// Candidate C: epoch-parameterized variant.
///
/// The epoch is explicit input to the workload so a future protocol can
/// regenerate parameters from chain state rather than relying on a permanent
/// fixed workload. Epoch derivation is intentionally simple at this stage.
pub fn work_candidate_c(
    seed: &[u8],
    nonce: u64,
    epoch: u64,
    config: Config,
) -> [u8; 32] {
    let mut epoch_seed = Vec::with_capacity(seed.len() + 8);
    epoch_seed.extend_from_slice(seed);
    epoch_seed.extend_from_slice(&epoch.to_le_bytes());
    work_candidate_b(&epoch_seed, nonce, config)
}

fn finalize(seed: &[u8], nonce: u64, state: u64, memory: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(seed);
    hasher.update(nonce.to_le_bytes());
    hasher.update(state.to_le_bytes());

    let sample_step = (memory.len() / 64).max(1);
    for value in memory.iter().step_by(sample_step).take(64) {
        hasher.update(value.to_le_bytes());
    }

    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let config = Config {
            memory_kib: 64,
            rounds: 2,
        };
        assert_eq!(
            work(b"memobi", 42, config),
            work(b"memobi", 42, config)
        );
    }

    #[test]
    fn nonce_changes_work() {
        let config = Config {
            memory_kib: 64,
            rounds: 2,
        };
        assert_ne!(
            work(b"memobi", 1, config),
            work(b"memobi", 2, config)
        );
    }

    #[test]
    fn zero_memory_is_normalized() {
        let zero = Config {
            memory_kib: 0,
            rounds: 1,
        };
        let one = Config {
            memory_kib: 1,
            rounds: 1,
        };
        assert_ne!(work(b"memobi", 1, zero), work(b"memobi", 1, one));
    }

    #[test]
    fn candidate_b_is_deterministic() {
        let config = Config {
            memory_kib: 64,
            rounds: 2,
        };
        assert_eq!(
            work_candidate_b(b"memobi", 42, config),
            work_candidate_b(b"memobi", 42, config)
        );
    }

    #[test]
    fn candidate_c_depends_on_epoch() {
        let config = Config {
            memory_kib: 64,
            rounds: 2,
        };
        assert_ne!(
            work_candidate_c(b"memobi", 42, 1, config),
            work_candidate_c(b"memobi", 42, 2, config)
        );
    }
}
