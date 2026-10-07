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

    /// Build an exponential-backoff locator from canonical headers.
    ///
    /// Newest headers are sampled densely, then progressively farther apart,
    /// with genesis always included. This lets a peer find a common ancestor
    /// without sending the entire local chain.
    pub fn from_headers(headers: &[crate::block::BlockHeader]) -> Self {
        if headers.is_empty() {
            return Self { hashes: Vec::new() };
        }

        let mut indexes = Vec::new();
        let mut index = headers.len() - 1;
        let mut step = 1usize;
        loop {
            indexes.push(index);
            if index == 0 {
                break;
            }
            index = index.saturating_sub(step);
            step = step.saturating_mul(2).max(1);
        }

        indexes.sort_unstable();
        indexes.dedup();
        let hashes = indexes
            .into_iter()
            .filter_map(|i| headers[i].block_id().ok())
            .rev()
            .collect();
        Self { hashes }
    }

    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    /// Return the first locator hash that exists in canonical headers.
    /// The result is the canonical height immediately after that ancestor.
    pub fn common_ancestor_start(
        &self,
        headers: &[crate::block::BlockHeader],
    ) -> Option<usize> {
        self.hashes.iter().find_map(|wanted| {
            headers.iter().position(|header| {
                header.block_id().ok().as_ref() == Some(wanted)
            })
        }).map(|index| index.saturating_add(1))
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
        // Peer height is an observation, not an instruction to roll the local
        // sync target backwards. A stale/replayed Version message must not make
        // an active synchronization session appear caught up.
        self.best_known_height = self.best_known_height.max(best_known_height);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncLimits {
    pub max_block_batch: u64,
}

impl Default for SyncLimits {
    fn default() -> Self {
        Self {
            max_block_batch: 256,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncRequest {
    GetHeaders { locator: HeaderLocator },
    GetBlocks { start_height: u64, count: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderSyncPlan {
    pub locator: HeaderLocator,
    pub local_height: u64,
    pub best_known_height: u64,
}

impl HeaderSyncPlan {
    pub fn new(
        headers: &[crate::block::BlockHeader],
        best_known_height: u64,
    ) -> Self {
        Self {
            locator: HeaderLocator::from_headers(headers),
            local_height: headers.last().map(|h| h.height.0).unwrap_or(0),
            best_known_height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlanner {
    pub limits: SyncLimits,
    pub progress: SyncProgress,
}

impl SyncPlanner {
    pub fn new(local_height: u64, best_known_height: u64, limits: SyncLimits) -> Self {
        Self {
            limits,
            progress: SyncProgress::new(local_height, best_known_height),
        }
    }

    pub fn next_request(&mut self, tip: Option<Hash32>) -> Option<SyncRequest> {
        self.next_request_with_locator(HeaderLocator::from_tip(tip))
    }

    pub fn plan_headers(
        &self,
        headers: &[crate::block::BlockHeader],
    ) -> HeaderSyncPlan {
        HeaderSyncPlan::new(headers, self.progress.best_known_height)
    }

    pub fn next_request_with_locator(&mut self, locator: HeaderLocator) -> Option<SyncRequest> {
        if self.progress.best_known_height <= self.progress.local_height {
            self.progress.state = SyncState::Synced;
            return None;
        }

        match self.progress.state {
            SyncState::HeaderSync | SyncState::Idle | SyncState::Synced => {
                self.progress.state = SyncState::HeaderSync;
                Some(SyncRequest::GetHeaders { locator })
            }
            SyncState::BlockSync => {
                let remaining = self.progress.best_known_height - self.progress.local_height;
                Some(SyncRequest::GetBlocks {
                    start_height: self.progress.local_height.saturating_add(1),
                    count: remaining.min(self.limits.max_block_batch),
                })
            }
        }
    }

    pub fn headers_received(&mut self, highest_height: u64) {
        self.progress
            .update_best_height(highest_height.max(self.progress.best_known_height));
        if self.progress.state != SyncState::Synced {
            self.progress.state = SyncState::BlockSync;
        }
    }

    pub fn blocks_applied(&mut self, highest_height: u64) {
        self.progress.advance_local_height(highest_height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_batches_blocks_and_stops_when_caught_up() {
        let mut planner = SyncPlanner::new(
            10,
            700,
            SyncLimits {
                max_block_batch: 64,
            },
        );
        assert!(matches!(
            planner.next_request(Some(Hash32([7; 32]))),
            Some(SyncRequest::GetHeaders { .. })
        ));
        planner.headers_received(700);
        assert_eq!(
            planner.next_request(Some(Hash32([7; 32]))),
            Some(SyncRequest::GetBlocks {
                start_height: 11,
                count: 64
            })
        );
        planner.blocks_applied(700);
        assert_eq!(planner.progress.state, SyncState::Synced);
        assert_eq!(planner.next_request(None), None);
    }

    #[test]
    fn planner_never_requests_more_than_remaining_blocks() {
        let mut planner = SyncPlanner::new(98, 100, SyncLimits::default());
        planner.headers_received(100);
        assert_eq!(
            planner.next_request(None),
            Some(SyncRequest::GetBlocks {
                start_height: 99,
                count: 2
            })
        );
    }

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
    fn locator_uses_exponential_backoff_and_includes_genesis() {
        let headers: Vec<_> = (0..10)
            .map(|height| crate::block::BlockHeader {
                version: 1,
                previous_block: Hash32([height.saturating_sub(1) as u8; 32]),
                height: crate::BlockHeight(height),
                timestamp: 1_700_000_000 + height * 10,
                target: u64::MAX,
                poarm_version: 0,
                poarm_nonce: height,
                transaction_root: Hash32([height as u8; 32]),
            })
            .collect();
        let locator = HeaderLocator::from_headers(&headers);
        assert_eq!(locator.hashes.len(), 5);
        assert_eq!(locator.hashes.last(), Some(&headers[0].block_id().unwrap()));
    }

    #[test]
    fn locator_can_start_from_tip() {
        let locator = HeaderLocator::from_tip(Some(Hash32([9; 32])));
        assert_eq!(locator.hashes, vec![Hash32([9; 32])]);
        assert!(!locator.is_empty());
    }
}


#[cfg(test)]
mod header_sync_plan_tests {
    use super::*;

    #[test]
    fn header_plan_tracks_canonical_height_and_locator() {
        let headers = vec![crate::block::BlockHeader {
            version: 1,
            previous_block: Hash32::ZERO,
            height: crate::block::BlockHeight(0),
            timestamp: 1,
            target: u64::MAX,
            poarm_version: 0,
            poarm_nonce: 0,
            transaction_root: Hash32::ZERO,
        }];
        let plan = HeaderSyncPlan::new(&headers, 8);
        assert_eq!(plan.local_height, 0);
        assert_eq!(plan.best_known_height, 8);
        assert_eq!(plan.locator.hashes.len(), 1);
    }
}
