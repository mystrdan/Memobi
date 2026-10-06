//! Provisional in-process node engine.
//!
//! Connects peer-session state, synchronization planning, consensus block
//! application, and optional canonical-header persistence without owning a
//! transport. Socket implementations can drive this engine later.

use crate::{
    chain::{Block, ChainError, ChainState},
    hash::Hash32,
    p2p::{Message, PeerSession},
    params::ConsensusParams,
    storage::HeaderStore,
    sync::{SyncLimits, SyncPlanner, SyncRequest},
};

#[derive(Debug)]
pub enum NodeError {
    Protocol(crate::ProtocolError),
    Chain(ChainError),
    NotEstablished,
}

impl From<crate::ProtocolError> for NodeError {
    fn from(value: crate::ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl From<ChainError> for NodeError {
    fn from(value: ChainError) -> Self {
        Self::Chain(value)
    }
}

pub struct Node {
    pub chain: ChainState,
    pub peer: PeerSession,
    pub sync: SyncPlanner,
    pub params: ConsensusParams,
}

impl Node {
    pub fn new(params: ConsensusParams) -> Self {
        Self {
            chain: ChainState::default(),
            peer: PeerSession::default(),
            sync: SyncPlanner::new(0, 0, SyncLimits::default()),
            params,
        }
    }

    pub fn start_peer(&mut self) -> Message {
        self.peer.start_with_height(self.chain.height.unwrap_or(0))
    }

    pub fn receive_peer_message(
        &mut self,
        message: Message,
        now_secs: u64,
        store: Option<&mut HeaderStore>,
    ) -> Result<Option<Message>, NodeError> {
        match message {
            Message::Version { .. }
            | Message::Verack
            | Message::Ping { .. }
            | Message::Pong { .. } => {
                let reply = self.peer.receive(message)?;
                if let Some(height) = self.peer.remote_height {
                    self.sync.progress.update_best_height(height);
                }
                Ok(reply)
            }
            Message::Headers { headers } => {
                if self.peer.phase != crate::p2p::PeerPhase::Established {
                    return Err(NodeError::NotEstablished);
                }
                let mut candidate = self.chain.clone();
                let has_headers = !headers.is_empty();
                let mut highest = candidate.height.unwrap_or(0);
                for raw in headers {
                    let header = Message::decode_header(&raw)?;
                    crate::validation::validate_header_with_params(
                        &candidate,
                        &header,
                        &self.params,
                    )
                    .map_err(|e| NodeError::Chain(ChainError::InvalidHeader(e)))?;
                    candidate.tip = Some(header.block_id()?);
                    candidate.height = Some(header.height.0);
                    candidate.work = crate::chainwork::WorkScore(
                        candidate
                            .work
                            .0
                            .saturating_add(crate::chainwork::block_work(header.target).0),
                    );
                    candidate.headers.push(header);
                    highest = header.height.0;
                }
                if has_headers {
                    self.sync.headers_received(highest);
                }
                Ok(None)
            }
            Message::Blocks { blocks } => {
                if self.peer.phase != crate::p2p::PeerPhase::Established {
                    return Err(NodeError::NotEstablished);
                }
                for envelope in blocks {
                    let (block, proof) = Message::decode_block(&envelope, &self.params)?;
                    self.apply_received_block(block, proof, now_secs, store.as_deref_mut())?;
                }
                Ok(None)
            }
            _ => Err(NodeError::Protocol(
                crate::ProtocolError::InvalidMessageType,
            )),
        }
    }

    pub fn apply_received_block(
        &mut self,
        block: Block,
        proof: Hash32,
        now_secs: u64,
        store: Option<&mut HeaderStore>,
    ) -> Result<Hash32, NodeError> {
        let parent_timestamp = self.chain.headers.last().map(|h| h.timestamp);
        let id = if let Some(store) = store {
            self.chain.apply_validated_block_with_store(
                &block,
                proof,
                &self.params,
                now_secs,
                parent_timestamp,
                store,
            )?
        } else {
            self.chain.apply_validated_block_with_params_and_context(
                &block,
                proof,
                &self.params,
                now_secs,
                parent_timestamp,
            )?
        };
        self.sync.blocks_applied(block.header.height.0);
        Ok(id)
    }

    pub fn apply_received_block_with_stores(
        &mut self,
        block: Block,
        proof: Hash32,
        now_secs: u64,
        header_store: &mut HeaderStore,
        block_store: &mut crate::storage::BlockStore,
    ) -> Result<Hash32, NodeError> {
        let parent_timestamp = self.chain.headers.last().map(|h| h.timestamp);
        let id = self.chain.apply_validated_block_with_stores(
            &block,
            proof,
            &self.params,
            now_secs,
            parent_timestamp,
            header_store,
            block_store,
        )?;
        self.sync.blocks_applied(block.header.height.0);
        Ok(id)
    }

    /// Restore a node from the canonical durable block log.
    ///
    /// Consensus validation is replayed rather than trusting persisted UTXO
    /// state, which keeps restart recovery auditable and privacy-friendly:
    /// only canonical chain data is required to reconstruct spendable state.
    pub fn recover_from_block_store(
        params: ConsensusParams,
        store: &mut crate::storage::BlockStore,
    ) -> Result<Self, NodeError> {
        let chain = ChainState::recover_from_block_store(&params, store)?;
        let height = chain.height.unwrap_or(0);
        Ok(Self {
            chain,
            peer: PeerSession::default(),
            sync: SyncPlanner::new(height, height, SyncLimits::default()),
            params,
        })
    }

    /// Serve a peer's bounded header request from the canonical header history.
    ///
    /// The first locator hash that matches our canonical chain becomes the
    /// starting point; headers after that point are returned in canonical
    /// height order. An empty locator starts at genesis.
    pub fn serve_get_headers(
        &self,
        locator: &[Hash32],
    ) -> Result<Message, NodeError> {
        const MAX_HEADERS_RESPONSE: usize = 2_000;
        let start = if locator.is_empty() {
            0
        } else {
            locator
                .iter()
                .find_map(|wanted| {
                    self.chain
                        .headers
                        .iter()
                        .position(|header| header.block_id().ok().as_ref() == Some(wanted))
                })
                .map(|index| index.saturating_add(1))
                .unwrap_or(0)
        };

        let headers = self
            .chain
            .headers
            .iter()
            .skip(start)
            .take(MAX_HEADERS_RESPONSE)
            .map(Message::encode_header)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Message::Headers { headers })
    }

    /// Serve a bounded block range from canonical durable storage.
    pub fn serve_get_blocks(
        &self,
        start_height: u64,
        count: u64,
        store: &mut crate::storage::BlockStore,
    ) -> Result<Message, NodeError> {
        const MAX_BLOCK_RESPONSE: u64 = 256;
        if count == 0 || count > MAX_BLOCK_RESPONSE {
            return Err(NodeError::Protocol(crate::ProtocolError::InvalidMessageSize));
        }

        let end_height = start_height
            .checked_add(count)
            .ok_or(NodeError::Protocol(crate::ProtocolError::InvalidMessageSize))?;
        let envelopes = store
            .read_all(&self.params)
            .map_err(|error| NodeError::Chain(ChainError::Storage(error)))?
            .into_iter()
            .filter(|(block, _)| {
                block.header.height.0 >= start_height && block.header.height.0 < end_height
            })
            .map(|(block, proof)| Message::encode_block(&block, proof))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Message::Blocks { blocks: envelopes })
    }

    pub fn next_sync_request(&mut self) -> Option<SyncRequest> {
        self.sync.next_request(self.chain.tip)
    }

    /// Translate the deterministic sync planner into the wire protocol.
    pub fn next_sync_message(&mut self) -> Option<Message> {
        match self.next_sync_request()? {
            SyncRequest::GetHeaders { locator } => Some(Message::GetHeaders {
                locator: locator.hashes,
            }),
            SyncRequest::GetBlocks {
                start_height,
                count,
            } => Some(Message::GetBlocks {
                start_height,
                count,
            }),
        }
    }

    pub fn observe_peer_height(&mut self, height: u64) {
        self.sync.headers_received(height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_starts_with_idle_peer_and_sync_state() {
        let node = Node::new(ConsensusParams::devnet());
        assert_eq!(node.peer.phase, crate::p2p::PeerPhase::Disconnected);
        assert_eq!(node.sync.progress.local_height, 0);
        assert_eq!(node.sync.progress.best_known_height, 0);
    }

    #[test]
    fn node_tracks_peer_height_after_version() {
        let mut node = Node::new(ConsensusParams::devnet());
        let reply = node
            .receive_peer_message(
                Message::Version {
                    protocol_version: 1,
                    node_nonce: 42,
                    height: 12,
                },
                0,
                None,
            )
            .unwrap();
        assert_eq!(reply, Some(Message::Verack));
        assert_eq!(node.peer.remote_height, Some(12));
        assert_eq!(node.sync.progress.best_known_height, 12);
    }

    #[test]
    fn node_accepts_a_real_genesis_envelope_after_handshake() {
        let mut node = Node::new(ConsensusParams::devnet());
        node.receive_peer_message(
            Message::Version {
                protocol_version: 1,
                node_nonce: 42,
                height: 0,
            },
            0,
            None,
        )
        .unwrap();

        let genesis = crate::genesis::devnet_genesis();
        let envelope = Message::encode_block(&genesis, Hash32::ZERO).unwrap();
        node.receive_peer_message(
            Message::Blocks {
                blocks: vec![envelope],
            },
            genesis.header.timestamp,
            None,
        )
        .unwrap();

        assert_eq!(node.chain.height, Some(0));
        assert_eq!(node.chain.tip, Some(genesis.header.block_id().unwrap()));
    }

    #[test]
    fn node_turns_header_plans_into_wire_messages() {
        let mut node = Node::new(ConsensusParams::devnet());
        node.sync.progress.update_best_height(10);
        assert!(matches!(
            node.next_sync_message(),
            Some(Message::GetHeaders { .. })
        ));
        node.sync.headers_received(10);
        assert_eq!(
            node.next_sync_message(),
            Some(Message::GetBlocks {
                start_height: 1,
                count: 10
            })
        );
    }

    #[test]
    fn node_validates_header_batches_before_block_sync() {
        let mut node = Node::new(ConsensusParams::devnet());
        node.receive_peer_message(
            Message::Version {
                protocol_version: 1,
                node_nonce: 42,
                height: 0,
            },
            0,
            None,
        )
        .unwrap();
        let genesis = crate::genesis::devnet_genesis();
        let header = Message::encode_header(&genesis.header).unwrap();
        node.receive_peer_message(
            Message::Headers {
                headers: vec![header],
            },
            0,
            None,
        )
        .unwrap();
        assert_eq!(node.sync.progress.state, crate::sync::SyncState::Synced);
    }

    #[test]
    fn node_serves_headers_after_locator() {
        let mut node = Node::new(ConsensusParams::devnet());
        let genesis = crate::genesis::devnet_genesis();
        node.apply_received_block(genesis.clone(), Hash32::ZERO, genesis.header.timestamp, None).unwrap();
        assert_eq!(node.serve_get_headers(&[]).unwrap(),
            Message::Headers { headers: vec![Message::encode_header(&genesis.header).unwrap()] });
        let locator = genesis.header.block_id().unwrap();
        assert_eq!(node.serve_get_headers(&[locator]).unwrap(),
            Message::Headers { headers: vec![] });
    }

    #[test]
    fn node_rejects_oversized_block_serving_request() {
        let node = Node::new(ConsensusParams::devnet());
        let dir = std::env::temp_dir().join(format!("memobi-node-serve-{}", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        let mut store = crate::storage::BlockStore::open(&dir).unwrap();
        let result = node.serve_get_blocks(0, 257, &mut store);
        assert!(matches!(result, Err(NodeError::Protocol(crate::ProtocolError::InvalidMessageSize))));
        let _ = std::fs::remove_file(dir);
    }

    #[test]
    fn node_rejects_blocks_before_handshake() {
        let mut node = Node::new(ConsensusParams::devnet());
        let result = node.receive_peer_message(Message::Blocks { blocks: vec![] }, 0, None);
        assert!(matches!(result, Err(NodeError::NotEstablished)));
    }
}
