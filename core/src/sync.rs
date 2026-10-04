//! Provisional chain-synchronization state primitives.
//!
//! Networking and peer scoring remain separate. This module only models the
//! deterministic state needed to request headers from a locator and track the
//! local synchronization phase.

use crate::Hash32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Idle,
    HeaderSync,
    BlockSync,
    Synced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderLocator {
    pub hashes: Vec<Hash32>,
}

impl HeaderLocator {
    pub fn from_tip(tip: Option<Hash32>) -> Self {
        let hashes = tip.into_iter().collect();
        Self { hashes }
    }

    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncProgress {
    pub state: SyncState,
    pub local_height: u64,
    pub best_known_height: u64,
}

impl SyncProgress {
    pub fn new(local_height: u64, best_known_height: u64) -> Self {
        let state = if best_known_height <= local_height {
            SyncState::Synced
        } else {
            SyncState::HeaderSync
        };
        Self {
            state,
            local_height,
            best_known_height,
        }
    }

    pub fn update_best_height(&mut self, best_known_height: u64) {
        self.best_known_height = best_known_height;
        if self.best_known_height <= self.local_height {
            self.state = SyncState::Synced;
        } else if self.state == SyncState::Idle || self.state == SyncState::Synced {
            self.state = SyncState::HeaderSync;
        }
    }

    pub fn advance_local_height(&mut self, height: u64) {
        self.local_height = self.local_height.max(height);
        if self.local_height >= self.best_known_height {
            self.state = SyncState::Synced;
        } else {
            self.state = SyncState::BlockSync;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_height_is_synced() {
        assert_eq!(SyncProgress::new(10, 10).state, SyncState::Synced);
    }

    #[test]
    fn higher_peer_height_starts_header_sync() {
        assert_eq!(SyncProgress::new(10, 20).state, SyncState::HeaderSync);
    }

    #[test]
    fn advancing_to_best_height_finishes_sync() {
        let mut progress = SyncProgress::new(10, 20);
        progress.advance_local_height(20);
        assert_eq!(progress.state, SyncState::Synced);
    }

    #[test]
    fn locator_can_start_from_tip() {
        let locator = HeaderLocator::from_tip(Some(Hash32([9; 32])));
        assert_eq!(locator.hashes, vec![Hash32([9; 32])]);
        assert!(!locator.is_empty());
    }
}
