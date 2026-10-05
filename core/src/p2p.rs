//! Provisional P2P message primitives.
//!
//! Message names and wire format are not consensus-frozen. This module exists
//! so networking work can proceed without coupling it to a centralized API.

use crate::{
    ProtocolError,
    codec::{Reader, put_bytes, put_u32_le},
    hash::Hash32,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Version {
        protocol_version: u32,
        node_nonce: u64,
        height: u64,
    },
    Verack,
    GetHeaders {
        locator: Vec<Hash32>,
    },
    Headers {
        headers: Vec<Vec<u8>>,
    },
    GetBlocks {
        locator: Vec<Hash32>,
    },
    Blocks {
        blocks: Vec<Vec<u8>>,
    },
    Inv {
        hashes: Vec<Hash32>,
    },
    Tx {
        transaction: Vec<u8>,
    },
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
}

const VERSION: u8 = 1;
const MAX_COLLECTION_ITEMS: usize = 4_096;
const MAX_MESSAGE_PAYLOAD_BYTES: usize = 2 * 1024 * 1024;

fn checked_count(count: u32) -> Result<usize, ProtocolError> {
    let count = count as usize;
    if count > MAX_COLLECTION_ITEMS {
        return Err(ProtocolError::InvalidMessageSize);
    }
    Ok(count)
}

fn read_bounded_bytes(reader: &mut Reader<'_>) -> Result<Vec<u8>, ProtocolError> {
    let len = reader.read_u32_le()? as usize;
    if len > MAX_MESSAGE_PAYLOAD_BYTES {
        return Err(ProtocolError::InvalidMessageSize);
    }
    Ok(reader.read_bytes(len)?.to_vec())
}

impl Message {
    pub fn encode_block(
        block: &crate::chain::Block,
        proof: Hash32,
    ) -> Result<Vec<u8>, ProtocolError> {
        let block_bytes = block.encode_to_vec()?;
        let mut out = Vec::with_capacity(32 + block_bytes.len());
        out.extend_from_slice(proof.as_bytes());
        crate::codec::put_bytes(&mut out, &block_bytes)?;
        Ok(out)
    }

    pub fn decode_block(
        bytes: &[u8],
        params: &crate::params::ConsensusParams,
    ) -> Result<(crate::chain::Block, Hash32), ProtocolError> {
        let mut reader = Reader::new(bytes);
        let proof = Hash32(reader.read_array()?);
        let block_bytes = crate::codec::read_bytes_u32(&mut reader)?;
        reader.finish()?;
        Ok((
            crate::chain::Block::decode_bounded(block_bytes, params)?,
            proof,
        ))
    }

    pub fn kind(&self) -> u8 {
        match self {
            Self::Version { .. } => 0,
            Self::Verack => 1,
            Self::GetHeaders { .. } => 2,
            Self::Headers { .. } => 3,
            Self::GetBlocks { .. } => 4,
            Self::Blocks { .. } => 5,
            Self::Inv { .. } => 6,
            Self::Tx { .. } => 7,
            Self::Ping { .. } => 8,
            Self::Pong { .. } => 9,
        }
    }

    pub fn encode_to_vec(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut out = Vec::new();
        out.push(VERSION);
        out.push(self.kind());

        match self {
            Self::Version {
                protocol_version,
                node_nonce,
                height,
            } => {
                put_u32_le(&mut out, *protocol_version);
                out.extend_from_slice(&node_nonce.to_le_bytes());
                out.extend_from_slice(&height.to_le_bytes());
            }
            Self::Verack => {}
            Self::GetHeaders { locator } | Self::GetBlocks { locator } => {
                put_u32_le(
                    &mut out,
                    u32::try_from(locator.len()).map_err(|_| ProtocolError::LengthOverflow)?,
                );
                for hash in locator {
                    out.extend_from_slice(hash.as_bytes());
                }
            }
            Self::Headers { headers } | Self::Blocks { blocks: headers } => {
                put_u32_le(
                    &mut out,
                    u32::try_from(headers.len()).map_err(|_| ProtocolError::LengthOverflow)?,
                );
                for header in headers {
                    put_bytes(&mut out, header)?;
                }
            }
            Self::Inv { hashes } => {
                put_u32_le(
                    &mut out,
                    u32::try_from(hashes.len()).map_err(|_| ProtocolError::LengthOverflow)?,
                );
                for hash in hashes {
                    out.extend_from_slice(hash.as_bytes());
                }
            }
            Self::Tx { transaction } => put_bytes(&mut out, transaction)?,
            Self::Ping { nonce } | Self::Pong { nonce } => {
                out.extend_from_slice(&nonce.to_le_bytes());
            }
        }

        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut reader = Reader::new(bytes);
        if reader.read_u8()? != VERSION {
            return Err(ProtocolError::InvalidMessageVersion);
        }

        let kind = reader.read_u8()?;
        let message = match kind {
            0 => Self::Version {
                protocol_version: reader.read_u32_le()?,
                node_nonce: u64::from_le_bytes(reader.read_array()?),
                height: u64::from_le_bytes(reader.read_array()?),
            },
            1 => Self::Verack,
            2 | 4 => {
                let count = checked_count(reader.read_u32_le()?)?;
                let mut locator = Vec::with_capacity(count);
                for _ in 0..count {
                    locator.push(Hash32(reader.read_array()?));
                }
                if kind == 2 {
                    Self::GetHeaders { locator }
                } else {
                    Self::GetBlocks { locator }
                }
            }
            3 | 5 => {
                let count = checked_count(reader.read_u32_le()?)?;
                let mut payloads = Vec::with_capacity(count);
                for _ in 0..count {
                    payloads.push(read_bounded_bytes(&mut reader)?);
                }
                if kind == 3 {
                    Self::Headers { headers: payloads }
                } else {
                    Self::Blocks { blocks: payloads }
                }
            }
            6 => {
                let count = checked_count(reader.read_u32_le()?)?;
                let mut hashes = Vec::with_capacity(count);
                for _ in 0..count {
                    hashes.push(Hash32(reader.read_array()?));
                }
                Self::Inv { hashes }
            }
            7 => Self::Tx {
                transaction: read_bounded_bytes(&mut reader)?,
            },
            8 | 9 => {
                let nonce = u64::from_le_bytes(reader.read_array()?);
                if kind == 8 {
                    Self::Ping { nonce }
                } else {
                    Self::Pong { nonce }
                }
            }
            _ => return Err(ProtocolError::InvalidMessageType),
        };

        reader.finish()?;
        Ok(message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerPhase {
    Disconnected,
    VersionSent,
    Established,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerSession {
    pub phase: PeerPhase,
    pub remote_nonce: Option<u64>,
    pub remote_height: Option<u64>,
}

impl Default for PeerSession {
    fn default() -> Self {
        Self {
            phase: PeerPhase::Disconnected,
            remote_nonce: None,
            remote_height: None,
        }
    }
}

impl PeerSession {
    pub fn start(&mut self) -> Message {
        self.start_with_height(0)
    }

    pub fn start_with_height(&mut self, height: u64) -> Message {
        self.phase = PeerPhase::VersionSent;
        Message::Version {
            protocol_version: 1,
            node_nonce: 0,
            height,
        }
    }

    pub fn receive(&mut self, message: Message) -> Result<Option<Message>, ProtocolError> {
        match message {
            Message::Version {
                protocol_version,
                node_nonce,
                height,
            } => {
                if protocol_version != 1 || self.remote_nonce == Some(node_nonce) {
                    return Err(ProtocolError::UnsupportedVersion);
                }
                self.remote_nonce = Some(node_nonce);
                self.remote_height = Some(height);
                self.phase = PeerPhase::Established;
                Ok(Some(Message::Verack))
            }
            Message::Verack if self.phase == PeerPhase::VersionSent => {
                self.phase = PeerPhase::Established;
                Ok(None)
            }
            Message::Ping { nonce } if self.phase == PeerPhase::Established => {
                Ok(Some(Message::Pong { nonce }))
            }
            Message::Pong { .. } if self.phase == PeerPhase::Established => Ok(None),
            _ => Err(ProtocolError::InvalidMessageType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_session_completes_version_handshake_and_ping() {
        let mut a = PeerSession::default();
        assert_eq!(a.phase, PeerPhase::Disconnected);
        assert!(matches!(a.start(), Message::Version { .. }));
        let reply = a.receive(Message::Verack).unwrap();
        assert_eq!(reply, None);
        assert_eq!(a.phase, PeerPhase::Established);
        assert_eq!(
            a.receive(Message::Ping { nonce: 55 }).unwrap(),
            Some(Message::Pong { nonce: 55 })
        );
    }

    #[test]
    fn peer_session_records_remote_version_height() {
        let mut peer = PeerSession::default();
        let reply = peer
            .receive(Message::Version {
                protocol_version: 1,
                node_nonce: 42,
                height: 123,
            })
            .unwrap();
        assert_eq!(reply, Some(Message::Verack));
        assert_eq!(peer.phase, PeerPhase::Established);
        assert_eq!(peer.remote_nonce, Some(42));
        assert_eq!(peer.remote_height, Some(123));
    }

    #[test]
    fn block_envelope_round_trips() {
        let block = crate::genesis::devnet_genesis();
        let proof = Hash32([7u8; 32]);
        let encoded = Message::encode_block(&block, proof).unwrap();
        let (decoded, decoded_proof) =
            Message::decode_block(&encoded, &crate::params::ConsensusParams::devnet()).unwrap();
        assert_eq!(decoded, block);
        assert_eq!(decoded_proof, proof);
    }

    #[test]
    fn oversized_collection_is_rejected() {
        let mut bytes = vec![VERSION, 2];
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Message::decode(&bytes),
            Err(ProtocolError::InvalidMessageSize)
        );
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let mut bytes = vec![VERSION, 7];
        bytes.extend_from_slice(&(2 * 1024 * 1024 + 1u32).to_le_bytes());
        assert_eq!(
            Message::decode(&bytes),
            Err(ProtocolError::InvalidMessageSize)
        );
    }

    #[test]
    fn message_round_trip() {
        let messages = [
            Message::Version {
                protocol_version: 1,
                node_nonce: 42,
                height: 7,
            },
            Message::Verack,
            Message::GetHeaders {
                locator: vec![Hash32([1u8; 32])],
            },
            Message::Headers {
                headers: vec![b"header".to_vec()],
            },
            Message::GetBlocks {
                locator: vec![Hash32([4u8; 32])],
            },
            Message::Blocks {
                blocks: vec![b"block".to_vec()],
            },
            Message::Inv {
                hashes: vec![Hash32([2u8; 32]), Hash32([3u8; 32])],
            },
            Message::Tx {
                transaction: b"tx".to_vec(),
            },
            Message::Ping { nonce: 99 },
            Message::Pong { nonce: 99 },
        ];

        for message in messages {
            let encoded = message.encode_to_vec().unwrap();
            assert_eq!(Message::decode(&encoded).unwrap(), message);
        }
    }
}
