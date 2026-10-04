//! Experimental PoARM laboratory implementation.
//!
//! This is a research benchmark and is NOT consensus-ready.
//! The workload is intentionally simple so that its performance and scaling
//! characteristics can be measured before a production PoARM design exists.

use sha2::{Digest, Sha256};

pub const VERSION: u32 = 0;
pub const DEFAULT_MEMORY_KIB: usize = 1024;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub memory_kib: usize,
    pub rounds: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            memory_kib: DEFAULT_MEMORY_KIB,
            rounds: 8,
        }
    }
}

pub fn work(seed: &[u8], nonce: u64, config: Config) -> [u8; 32] {
    let words = (config.memory_kib.max(1) * 1024) / 8;
    let mut memory = vec![0u64; words];

    let mut input = Vec::with_capacity(seed.len() + 8);
    input.extend_from_slice(seed);
    input.extend_from_slice(&nonce.to_le_bytes());

    let digest = Sha256::digest(&input);

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
        assert_eq!(work(b"memobi", 42, config), work(b"memobi", 42, config));
    }

    #[test]
    fn nonce_changes_work() {
        let config = Config {
            memory_kib: 64,
            rounds: 2,
        };
        assert_ne!(work(b"memobi", 1, config), work(b"memobi", 2, config));
    }
}
