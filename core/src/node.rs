//! Provisional in-process node engine.
//!
//! Connects peer-session state, synchronization planning, consensus block
//! application, and optional canonical-header persistence without owning a
//! transport. Socket implementations can drive this engine later.

use crate::{
    chain::{Block, ChainError, ChainState},
    mempool::{Mempool, MempoolError},
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
    Transport(crate::transport::TransportError),
    NotEstablished,
    Mempool(MempoolError),
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

impl From<MempoolError> for NodeError {
    fn from(value: MempoolError) -> Self {
        Self::Mempool(value)
    }
}

impl From<crate::transport::TransportError> for NodeError {
    fn from(value: crate::transport::TransportError) -> Self {
        Self::Transport(value)
    }
}

pub struct Node {
    pub chain: ChainState,
    pub mempool: Mempool,
    pub peer: PeerSession,
    pub sync: SyncPlanner,
    pub params: ConsensusParams,
}

impl Node {
    pub fn new(params: ConsensusParams) -> Self {
        Self {
            chain: ChainState::default(),
            mempool: Mempool::new(),
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
            Message::Tx { transaction } => {
                if self.peer.phase != crate::p2p::PeerPhase::Established {
                    return Err(NodeError::NotEstablished);
                }
                self.accept_transaction_bytes(&transaction, now_secs, 0)?;
                Ok(None)
            }
            Message::Blocks { blocks } => {
                if self.peer.phase != crate::p2p::PeerPhase::Established {
                    return Err(NodeError::NotEstablished);
                }
                let mut decoded = Vec::with_capacity(blocks.len());
                for envelope in blocks {
                    decoded.push(Message::decode_block(&envelope, &self.params)?);
                }
                if store.is_none() {
                    self.apply_received_blocks(&decoded, now_secs)?;
                } else {
                    for (block, proof) in decoded {
                        self.apply_received_block(block, proof, now_secs, store.as_deref_mut())?;
                    }
                }
                Ok(None)
            }
            _ => Err(NodeError::Protocol(
                crate::ProtocolError::InvalidMessageType,
            )),
        }
    }

    /// Validate a complete block batch against a staged chain before committing.
    /// This prevents a later invalid block from partially advancing an in-memory node.
    /// Accept a transaction locally with consensus validation and mempool policy.
    /// This path is independent of block confirmation for low-latency UX.
    pub fn submit_transaction(
        &mut self,
        transaction: crate::Transaction,
        now_secs: u64,
        min_fee: u64,
    ) -> Result<Hash32, NodeError> {
        let tip_height = self.chain.height.unwrap_or(0);
        Ok(self.mempool.insert(
            transaction,
            &self.chain.utxos,
            &self.params,
            tip_height,
            now_secs,
            min_fee,
        )?)
    }

    /// Decode and accept a canonical transaction received from a peer.
    pub fn accept_transaction_bytes(
        &mut self,
        bytes: &[u8],
        now_secs: u64,
        min_fee: u64,
    ) -> Result<Hash32, NodeError> {
        let transaction = crate::Transaction::decode_bounded(bytes, &self.params)?;
        self.submit_transaction(transaction, now_secs, min_fee)
    }

    /// Build a relay message for an already-accepted transaction.
    pub fn transaction_message(&self, txid: &Hash32) -> Option<Message> {
        if !self.mempool.contains(txid) {
            return None;
        }
        self.mempool.candidates(self.mempool.len()).into_iter().find_map(|tx| {
            tx.txid().ok().filter(|id| id == txid).and_then(|_| {
                tx.encode_to_vec().ok().map(|transaction| Message::Tx { transaction })
            })
        })
    }

    pub fn apply_received_blocks(
        &mut self,
        blocks: &[(Block, Hash32)],
        now_secs: u64,
    ) -> Result<(), NodeError> {
        if blocks.is_empty() {
            return Err(NodeError::Protocol(crate::ProtocolError::InvalidMessageSize));
        }
        let mut staged = self.chain.clone();
        for (block, proof) in blocks {
            let parent_timestamp = staged.headers.last().map(|h| h.timestamp);
            staged.apply_validated_block_with_params_and_context(
                block,
                *proof,
                &self.params,
                now_secs,
                parent_timestamp,
            )?;
        }
        let height = staged.height.unwrap_or(0);
        self.chain = staged;
        self.sync.blocks_applied(height);
        Ok(())
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

    /// Restore a node only when the durable header and block logs agree.
    /// Validate a complete competing branch from genesis and atomically
    /// replace the canonical chain only when cumulative work wins.
    pub fn apply_received_fork(
        &mut self,
        fork: &[(Block, Hash32)],
    ) -> Result<bool, NodeError> {
        let replaced = self.chain.replay_fork_with_params(fork, &self.params)?;
        if replaced {
            let height = self.chain.height.unwrap_or(0);
            self.sync = SyncPlanner::new(height, height, SyncLimits::default());
        }
        Ok(replaced)
    }

    pub fn recover_from_stores(
        params: ConsensusParams,
        headers: &mut crate::storage::HeaderStore,
        blocks: &mut crate::storage::BlockStore,
    ) -> Result<Self, NodeError> {
        crate::storage::verify_chain_consistency(headers, blocks, &params)
            .map_err(ChainError::Storage)?;
        Self::recover_from_block_store(params, blocks)
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
    /// Find where a peer's locator joins this canonical header chain.
    /// This is the synchronization boundary used before a future fork replay.
    pub fn common_ancestor_height(&self, locator: &[Hash32]) -> Option<u64> {
        let locator = crate::sync::HeaderLocator {
            hashes: locator.to_vec(),
        };
        locator
            .common_ancestor_start(&self.chain.headers)
            .and_then(|next| next.checked_sub(1))
            .and_then(|index| self.chain.headers.get(index))
            .map(|header| header.height.0)
    }

    pub fn serve_get_headers(&self, locator: &[Hash32]) -> Result<Message, NodeError> {
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
            return Err(NodeError::Protocol(
                crate::ProtocolError::InvalidMessageSize,
            ));
        }

        start_height.checked_add(count).ok_or(NodeError::Protocol(
            crate::ProtocolError::InvalidMessageSize,
        ))?;
        let envelopes = store
            .read_range(&self.params, start_height, count)
            .map_err(|error| NodeError::Chain(ChainError::Storage(error)))?
            .into_iter()
            .map(|(block, proof)| Message::encode_block(&block, proof))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Message::Blocks { blocks: envelopes })
    }

    /// Handle a decoded synchronization request at the node boundary.
    ///
    /// Transport remains external: it supplies the decoded request and receives
    /// the canonical response generated from local chain/storage state.
    pub fn serve_sync_request(
        &self,
        message: Message,
        block_store: &mut crate::storage::BlockStore,
    ) -> Result<Message, NodeError> {
        if self.peer.phase != crate::p2p::PeerPhase::Established {
            return Err(NodeError::NotEstablished);
        }
        match message {
            Message::GetHeaders { locator } => self.serve_get_headers(&locator),
            Message::GetBlocks {
                start_height,
                count,
            } => self.serve_get_blocks(start_height, count, block_store),
            _ => Err(NodeError::Protocol(
                crate::ProtocolError::InvalidMessageType,
            )),
        }
    }

    /// Send the local handshake message through an established TCP transport.
    pub fn start_tcp_peer(
        &mut self,
        transport: &mut crate::transport::TcpPeer,
    ) -> Result<(), NodeError> {
        transport.send(&self.start_peer())?;
        Ok(())
    }

    /// Receive one framed TCP message, feed it through the node engine, and
    /// send any protocol reply produced by the peer session.
    pub fn receive_tcp_message(
        &mut self,
        transport: &mut crate::transport::TcpPeer,
        now_secs: u64,
        store: Option<&mut HeaderStore>,
    ) -> Result<(), NodeError> {
        let message = transport.receive()?;
        if let Some(reply) = self.receive_peer_message(message, now_secs, store)? {
            transport.send(&reply)?;
        }
        Ok(())
    }

    /// Receive one framed TCP message while preserving both durable stores.
    ///
    /// The lightweight TCP helper remains available, but this variant is the
    /// persistence-safe boundary for a canonical node: received blocks are
    /// committed through the dual-store chain path rather than only the header log.
    pub fn receive_tcp_message_with_stores(
        &mut self,
        transport: &mut crate::transport::TcpPeer,
        now_secs: u64,
        header_store: &mut HeaderStore,
        block_store: &mut crate::storage::BlockStore,
    ) -> Result<(), NodeError> {
        let message = transport.receive()?;
        match message {
            Message::GetHeaders { locator } => {
                let reply = self.serve_sync_request(
                    Message::GetHeaders { locator },
                    block_store,
                )?;
                transport.send(&reply)?;
            }
            Message::GetBlocks {
                start_height,
                count,
            } => {
                let reply = self.serve_sync_request(
                    Message::GetBlocks {
                        start_height,
                        count,
                    },
                    block_store,
                )?;
                transport.send(&reply)?;
            }
            Message::Blocks { blocks } => {
                if self.peer.phase != crate::p2p::PeerPhase::Established {
                    return Err(NodeError::NotEstablished);
                }
                if blocks.is_empty() {
                    return Err(NodeError::Protocol(
                        crate::ProtocolError::InvalidMessageSize,
                    ));
                }
                let decoded = blocks
                    .iter()
                    .map(|envelope| Message::decode_block(envelope, &self.params))
                    .collect::<Result<Vec<_>, _>>()?;
                for (block, proof) in decoded {
                    self.apply_received_block_with_stores(
                        block,
                        proof,
                        now_secs,
                        header_store,
                        block_store,
                    )?;
                }
            }
            other => {
                if let Some(reply) =
                    self.receive_peer_message(other, now_secs, Some(header_store))?
                {
                    transport.send(&reply)?;
                }
            }
        }
        Ok(())
    }

    pub fn next_sync_request(&mut self) -> Option<SyncRequest> {
        let locator = crate::sync::HeaderLocator::from_headers(&self.chain.headers);
        self.sync.next_request_with_locator(locator)
    }

    /// Drive one request/response step over a persistent TCP peer.
    ///
    /// This is intentionally one framed message at a time so callers can
    /// choose their own event loop, timeout policy, and peer scheduling.
    pub fn receive_tcp_sync_step(
        &mut self,
        transport: &mut crate::transport::TcpPeer,
        now_secs: u64,
        header_store: &mut HeaderStore,
        block_store: &mut crate::storage::BlockStore,
    ) -> Result<(), NodeError> {
        self.receive_tcp_message_with_stores(
            transport,
            now_secs,
            header_store,
            block_store,
        )
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
    fn node_fork_replacement_is_atomic_and_resets_sync_state() {
        let mut node = Node::new(ConsensusParams::devnet());
        let genesis = crate::genesis::devnet_genesis();
        node.apply_received_block(
            genesis.clone(),
            Hash32::ZERO,
            genesis.header.timestamp,
            None,
        )
        .unwrap();

        let mut fork = vec![(genesis.clone(), Hash32::ZERO)];
        let mut previous = genesis.header.block_id().unwrap();
        for height in 1..=2 {
            let template = crate::block_builder::BlockTemplate {
                version: node.params.block_version,
                previous_block: previous,
                height: crate::BlockHeight(height),
                timestamp: genesis.header.timestamp + height * 10,
                target: node.params.max_target,
                poarm_version: node.params.poarm_version,
                poarm_nonce: height,
            };
            let block = template
                .build_mining_block_with_params(
                    Vec::new(),
                    vec![0u8; crate::crypto::PUBKEY_LEN],
                    &node.params,
                )
                .unwrap();
            previous = block.header.block_id().unwrap();
            fork.push((block, Hash32::ZERO));
        }

        assert!(node.apply_received_fork(&fork).unwrap());
        assert_eq!(node.chain.height, Some(2));
        assert_eq!(node.sync.progress.state, crate::sync::SyncState::Synced);
    }

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
        node.apply_received_block(
            genesis.clone(),
            Hash32::ZERO,
            genesis.header.timestamp,
            None,
        )
        .unwrap();
        assert_eq!(
            node.serve_get_headers(&[]).unwrap(),
            Message::Headers {
                headers: vec![Message::encode_header(&genesis.header).unwrap()]
            }
        );
        let locator = genesis.header.block_id().unwrap();
        assert_eq!(
            node.serve_get_headers(&[locator]).unwrap(),
            Message::Headers { headers: vec![] }
        );
    }

    #[test]
    fn node_serves_sync_request_only_after_handshake() {
        let mut node = Node::new(ConsensusParams::devnet());
        let dir = std::env::temp_dir().join(format!("memobi-node-request-{}", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        let mut store = crate::storage::BlockStore::open(&dir).unwrap();

        let before = node.serve_sync_request(Message::GetHeaders { locator: vec![] }, &mut store);
        assert!(matches!(before, Err(NodeError::NotEstablished)));

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

        let response = node
            .serve_sync_request(Message::GetHeaders { locator: vec![] }, &mut store)
            .unwrap();
        assert_eq!(response, Message::Headers { headers: vec![] });

        let _ = std::fs::remove_file(dir);
    }

    #[test]
    fn node_rejects_oversized_block_serving_request() {
        let node = Node::new(ConsensusParams::devnet());
        let dir = std::env::temp_dir().join(format!("memobi-node-serve-{}", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        let mut store = crate::storage::BlockStore::open(&dir).unwrap();
        let result = node.serve_get_blocks(0, 257, &mut store);
        assert!(matches!(
            result,
            Err(NodeError::Protocol(
                crate::ProtocolError::InvalidMessageSize
            ))
        ));
        let _ = std::fs::remove_file(dir);
    }

    #[test]
    fn node_block_batch_failure_does_not_partially_commit() {
        let params = ConsensusParams::devnet();
        let mut node = Node::new(params);
        let genesis = crate::genesis::devnet_genesis();
        node.apply_received_block(
            genesis.clone(),
            Hash32::ZERO,
            genesis.header.timestamp,
            None,
        )
        .unwrap();
        let bad = genesis.clone();
        let result = node.apply_received_blocks(
            &[
                (genesis.clone(), Hash32::ZERO),
                (bad, Hash32::ZERO),
            ],
            genesis.header.timestamp,
        );
        assert!(result.is_err());
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
