//! Provisional P2P message primitives.
//!
//! Message names and wire format are not consensus-frozen. This module exists
//! so networking work can proceed without coupling it to a centralized API.

use crate::{codec::{put_bytes, put_u32_le, Encode, Reader}, hash::Hash32, ProtocolError};

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

impl Message {
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
            Self::Headers { headers } | Self::Blocks { blocks } => {
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
                let count = reader.read_u32_le()? as usize;
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
                let count = reader.read_u32_le()? as usize;
                let mut payloads = Vec::with_capacity(count);
                for _ in 0..count {
                    payloads.push(crate::codec::read_bytes_u32(&mut reader)?.to_vec());
                }
                if kind == 3 {
                    Self::Headers { headers: payloads }
                } else {
                    Self::Blocks { blocks: payloads }
                }
            }
            6 => {
                let count = reader.read_u32_le()? as usize;
                let mut hashes = Vec::with_capacity(count);
                for _ in 0..count {
                    hashes.push(Hash32(reader.read_array()?));
                }
                Self::Inv { hashes }
            }
            7 => Self::Tx {
                transaction: crate::codec::read_bytes_u32(&mut reader)?.to_vec(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
