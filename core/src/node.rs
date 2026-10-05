//! Provisional in-process node engine.
//!
//! Connects peer-session state, synchronization planning, consensus block
//! application, and optional canonical-header persistence without owning a
//! transport. Socket implementations can drive this engine later.

use crate::{
    chain::{Block, ChainError, ChainState},
    hash::Hash32,
    params::ConsensusParams,
    p2p::{Message, PeerSession},
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
        self.peer.start()
    }

    pub fn receive_peer_message(
        &mut self,
        message: Message,
        now_secs: u64,
        store: Option<&mut HeaderStore>,
    ) -> Result<Option<Message>, NodeError> {
        match message {
            Message::Version { .. } | Message::Verack | Message::Ping { .. } | Message::Pong { .. } => {
                let reply = self.peer.receive(message)?;
                if let Some(height) = self.peer.remote_height {
                    self.sync.progress.update_best_height(height);
                }
                Ok(reply)
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
            _ => Err(NodeError::Protocol(crate::ProtocolError::InvalidMessageType)),
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
            self.chain
                .apply_validated_block_with_store(
                    &block,
                    proof,
                    &self.params,
                    now_secs,
                    parent_timestamp,
                    store,
                )?
        } else {
            self.chain
                .apply_validated_block_with_params_and_context(
                    &block,
                    proof,
                    &self.params,
                    now_secs,
                    parent_timestamp,
                )?
        };
        self.sync
            .blocks_applied(block.header.height.0);
        Ok(id)
    }

    pub fn next_sync_request(&mut self) -> Option<SyncRequest> {
        self.sync.next_request(self.chain.tip)
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
        ).unwrap();

        let genesis = crate::genesis::devnet_genesis();
        let envelope = Message::encode_block(&genesis, Hash32::ZERO).unwrap();
        node.receive_peer_message(
            Message::Blocks { blocks: vec![envelope] },
            genesis.header.timestamp,
            None,
        ).unwrap();

        assert_eq!(node.chain.height, Some(0));
        assert_eq!(node.chain.tip, Some(genesis.header.block_id().unwrap()));
    }

    #[test]
    fn node_rejects_blocks_before_handshake() {
        let mut node = Node::new(ConsensusParams::devnet());
        let result = node.receive_peer_message(Message::Blocks { blocks: vec![] }, 0, None);
        assert!(matches!(result, Err(NodeError::NotEstablished)));
    }
}
